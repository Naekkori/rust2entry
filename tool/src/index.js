'use strict';
// entryjs 하드웨어 블럭 스키마 → hw_sourcemap.json 생성.
// entryc 가 파싱할 수 있도록 장치별 블럭 스키마를 추출한다.

const { createContext } = require('./loader');

/**
 * entryjs 체크아웃에서 하드웨어 소스맵을 생성한다.
 * @param {string} entryjsRoot entryjs 경로
 * @returns {{sourcemap:object, stats:{devices:number, blocks:number, loaded:number, failed:number}}}
 */
function generate(entryjsRoot) {
  const ctx = createContext(entryjsRoot);
  const devices = [];
  let blocksTotal = 0;
  let loaded = 0;
  let failed = 0;

  for (const f of ctx.hardwareFiles()) {
    const name = f.replace(/\.js$/, '');
    try {
      const blocks = ctx.loadHardwareBlocks(name);
      const n = Object.keys(blocks).length;
      blocksTotal += n;
      loaded += 1;
      devices.push({
        name,
        file: 'hardware/' + f,
        blockCount: n,
        blocks: ctx.capture(blocks),
      });
    } catch {
      failed += 1;
      devices.push({ name, file: 'hardware/' + f, blockCount: 0, blocks: {} });
    }
  }

  const sourcemap = {
    generated: new Date().toISOString().slice(0, 10),
    source: 'entrylabs/entryjs',
    deviceCount: devices.length,
    blockTotal: blocksTotal,
    loaded,
    failed,
    devices,
  };
  return { sourcemap, stats: { devices: devices.length, blocks: blocksTotal, loaded, failed } };
}

module.exports = { generate };
