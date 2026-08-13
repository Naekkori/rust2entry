'use strict';
// entryjs 소스 로더 하네스.
//
// entryjs 블럭 파일들은 브라우저 전역(Entry, EntryStatic, Lang, ...)과
// webpack 전용 구문(import + module.exports 혼용, require.context)을 쓴다.
// 이 하네스는 그런 파일들을 Node 에서 로드할 수 있도록 전역 스텁과
// Module._load 인터셉트를 구성한다.

const fs = require('fs');
const Module = require('module');
const path = require('path');

/** 어떤 프로퍼티 접근/호출/할당도 흡수하는 유연한 프록시. */
function flexibleProxy() {
  const target = function () {};
  return new Proxy(target, {
    get: (t, p) =>
      p === Symbol.toPrimitive
        ? () => ''
        : p in t
          ? t[p]
          : (t[p] = flexibleProxy()),
    set: (t, p, v) => {
      t[p] = v;
      return true;
    },
    apply: () => flexibleProxy(),
  });
}

/**
 * tmp 변환 모듈의 상대 require 를 원본 소스 디렉터리 기준으로 해석한다.
 * 대소문자 정확 매칭 우선, 실패 시 대소문자 무시 fallback.
 * @param {string} baseDir 원본 소스 디렉터리
 * @param {string} request 상대 require 경로 ('./...' / '../...')
 * @returns {string|null} 해석된 절대 경로 (없으면 null)
 */
function resolveRelative(baseDir, request) {
  const target = path.resolve(baseDir, request);
  const tryPaths = [target, target + '.js', target + '.json', path.join(target, 'index.js')];
  for (const p of tryPaths) {
    if (fs.existsSync(p)) return p;
  }
  // 대소문자 무시 fallback: 대상 파일이 있는 디렉터리를 훑어 소문자 기준으로 일치하는 파일 찾기.
  const dir = path.dirname(target);
  const base = path.basename(target);
  if (fs.existsSync(dir)) {
    let hit;
    try {
      hit = fs.readdirSync(dir).find((f) => f.toLowerCase() === base.toLowerCase());
    } catch {
      hit = null;
    }
    if (hit) return path.join(dir, hit);
  }
  return null;
}

/** Node 24에서 getter-only 전역(navigator 등)도 안전하게 정의. */
function defineGlobal(name, value) {
  try {
    global[name] = value;
  } catch {
    try {
      Object.defineProperty(global, name, {
        value,
        writable: true,
        configurable: true,
      });
    } catch {
      /* 무시 */
    }
  }
}

/** tmp→소스 디렉터리 역방향 매핑 (상대 require 해석용). */
const TMP_REGISTRY = new Map();

/**
 * Lang 을 안전하게 감싼다:
 * - `Lang.template` — 존재하지 않는 키에 '' (charCodeAt/indexOf 등 문자열 연산 안전, 루프 종결 보장)
 * - `Lang.Blocks` — 존재하지 않는 키에 flexibleProxy (setLanguage 로만 채워지는 네임스페이스의 중첩 접근 안전)
 */
function wrapLangTemplate(lang) {
  if (lang && lang.template && typeof lang.template === 'object') {
    Object.defineProperty(lang, 'template', {
      value: new Proxy(lang.template, {
        get: (t, p) =>
          typeof p === 'string' && !(p in t) ? '' : t[p],
      }),
      configurable: true,
      enumerable: true,
    });
  }
  if (lang && lang.Blocks && typeof lang.Blocks === 'object') {
    Object.defineProperty(lang, 'Blocks', {
      value: new Proxy(lang.Blocks, {
        get: (t, p) =>
          typeof p === 'string' && !(p in t) ? flexibleProxy() : t[p],
      }),
      configurable: true,
      enumerable: true,
    });
  }
  return lang;
}

/**
 * 로더 컨텍스트를 생성한다.
 * @param {string} root entryjs 체크아웃 경로
 */
function createContext(root) {
  const ROOT = path.resolve(root);
  global.Lang = wrapLangTemplate(require(path.join(ROOT, 'extern/lang/ko.js')).Lang);
  global.EntryStatic = flexibleProxy();
  global.Entry = flexibleProxy();
  global._ = require(path.join(ROOT, 'node_modules/lodash'));
  defineGlobal('window', global);
  defineGlobal('navigator', { userAgent: 'node' });
  defineGlobal('document', {
    createElement: () => ({ getContext: () => flexibleProxy() }),
    addEventListener: () => {},
    body: { appendChild: () => {} },
    querySelector: () => null,
    getElementById: () => null,
  });
  defineGlobal('requestAnimationFrame', (f) => f);
  defineGlobal('cancelAnimationFrame', () => {});

  const STUB_LODASH = ['three', 'katex', 'isomorphic-fetch', 'cuid', 'uid'];
  const origLoad = Module._load;
  Module._load = function (request, parent, isMain) {
    if (request.startsWith('lodash/')) {
      return origLoad.call(this, request, { paths: [ROOT + '/node_modules'] }, isMain);
    }
    if (request.includes('inputs/keyboard')) {
      let s = fs.readFileSync(
        path.join(ROOT, 'src/playground/blocks/inputs/keyboard.js'),
        'utf8'
      );
      s = s.replace('export const keyInputList', 'const keyInputList') + '\nmodule.exports={keyInputList};\n';
      const m = new Module(request, parent);
      m.filename = request;
      m.paths = parent.paths;
      m._compile(s, request);
      return m.exports;
    }
    if (request.includes('class/DataTable')) {
      return class DataTable {
        static getSource() { return { fields: [] }; }
        static getColumnIndex() { return 0; }
        static saveTable() {}
        static getTables() { return []; }
      };
    }
    if (request.includes('class/entryModuleLoader')) {
      return { moduleList: [], moduleListLite: [] };
    }
    // tmp 변환 모듈의 상대 require → 원본 소스 디렉터리 기준으로 해석 (대소문자 무시 fallback 포함).
    if (
      (request.startsWith('./') || request.startsWith('../')) &&
      parent &&
      parent.filename &&
      TMP_REGISTRY.has(parent.filename)
    ) {
      const resolved = resolveRelative(TMP_REGISTRY.get(parent.filename), request);
      if (resolved) {
        return origLoad.call(this, resolved, parent, isMain);
      }
    }
    if (request.includes('util/common')) {
      return { toNumber: (v) => Number(v), toQueryString: () => '', callApi: () => Promise.resolve({}), getQueryString: () => ({}) };
    }
    if (request.includes('graphicEngine/GEHelper')) {
      return { hitTestMouse: () => null, getTransformedBounds: () => null };
    }
    if (request.includes('class/learning/Svm')) {
      return { classes: [], KERNEL_STRING_TYPE: { LINEAR: 'linear', POLYNOMIAL: 'polynomial', RBF: 'rbf', SIGMOID: 'sigmoid' }, OPTION_DEFAULT_VALUE: {} };
    }
    if (request.includes('util/mediaPipeUtils')) {
      const o = flexibleProxy(); o.getInstance = () => flexibleProxy(); o.flipState = {}; o.getInputList = async () => []; return o;
    }
    if (request.includes('util/audioUtils')) { const o = flexibleProxy(); o.default = o; return o; }
    if (request.includes('core/promiseManager')) { const o = flexibleProxy(); o.run = () => Promise.resolve(); o.prototype = { run: () => Promise.resolve() }; return o; }
    if (request.includes('util/location')) return { locationData: [], getStateOptions: () => [], getCityOptions: () => () => [] };
    if (request.includes('@entrylabs/legacy-video')) { const o = flexibleProxy(); o.default = o; return o; }
    if (!request.startsWith('.') && !request.startsWith('/') && !request.startsWith('node:') && STUB_LODASH.some((s) => request.includes(s))) {
      return flexibleProxy();
    }
    return origLoad.apply(this, arguments);
  };

  /** 블럭 파일을 import→require 변환 후 로드한다. */
  function loadFile(name, dir) {
    const base = dir
      ? path.join(ROOT, 'src/playground/blocks', dir)
      : path.join(ROOT, 'src/playground/blocks');
    let src = fs.readFileSync(path.join(base, name + '.js'), 'utf8');
    if (/^import /m.test(src)) {
      src = src.replace(
        /^import\s+([A-Za-z_$][\w$]*),\s*\{([^}]+)\}\s*from\s+'([^']+)';/gm,
        'const $1 = require("$3"); const { $2 } = require("$3");'
      );
      src = src.replace(
        /^import\s+([A-Za-z_$][\w$]*)\s+from\s+'([^']+)';\s*$/gm,
        'const $1 = require("$2");'
      );
      src = src.replace(
        /^import\s*\{([^}]+)\}\s*from\s+'([^']+)';\s*$/gm,
        'const { $1 } = require("$2");'
      );
    }
    const tmpDir = path.join(require('os').tmpdir(), 'entryjs-sm-conv');
    fs.mkdirSync(tmpDir, { recursive: true });
    const out = path.join(tmpDir, (dir ? dir + '_' : '') + name + '.js');
    fs.writeFileSync(out, src);
    TMP_REGISTRY.set(out, base);
    return require(out);
  }

  /** 블럭 정의 객체를 스키마 부분집합으로 축약한다. */
  function capture(blocks) {
    const out = {};
    for (const id of Object.keys(blocks)) {
      const d = blocks[id] || {};
      const def = d.def || {};
      out[id] = {
        skeleton: typeof d.skeleton === 'string' ? d.skeleton : null,
        color: typeof d.color === 'string' ? d.color : d.color ? String(d.color) : null,
        outerLine: typeof d.outerLine === 'string' ? d.outerLine : d.outerLine ? String(d.outerLine) : null,
        def_type: def.type !== undefined ? String(def.type) : null,
        has_func: typeof d.func === 'function',
        class: typeof d.class === 'string' ? d.class : null,
        params: Array.isArray(d.params)
          ? d.params.map((p) => (p && typeof p.type === 'string' ? p.type : null))
          : null,
        paramCount: Array.isArray(d.params) ? d.params.length : 0,
      };
    }
    return out;
  }

  /** 하드웨어 디렉터리 파일명 목록. */
  function hardwareFiles() {
    const dir = path.join(ROOT, 'src/playground/blocks/hardware');
    return fs
      .readdirSync(dir)
      .filter((x) => x.endsWith('.js') && x !== 'index.js')
      .sort();
  }

  /** 하드웨어 장치 하나를 로드해 그 블럭들을 반환한다. */
  function loadHardwareBlocks(name) {
    // 공유 base 모듈 상태 오염 방지: 일부 하드웨어 모듈이 `global.Entry = {}` 로
    // 프록시를 통째로 교체하므로, 장치별로 fresh 프록시를 다시 심어 로드 순서와 무관하게 만든다.
    global.Entry = flexibleProxy();
    global.EntryStatic = flexibleProxy();
    const mod = loadFile(name, 'hardware');
    if (mod && typeof mod.getBlocks === 'function') return mod.getBlocks();
    const cands = Object.keys(global.Entry).filter(
      (k) => global.Entry[k] && typeof global.Entry[k].getBlocks === 'function'
    );
    if (cands.length) {
      const b = {};
      for (const k of cands) Object.assign(b, global.Entry[k].getBlocks());
      return b;
    }
    return {};
  }

  return { ROOT, loadFile, capture, hardwareFiles, loadHardwareBlocks };
}

module.exports = { createContext };
