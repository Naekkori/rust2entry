'use strict';
// CLI 진입: entryjs 소스맵 생성기.
//
//   node tool/bin/entryjs-sourcemap.js --src <entryjs 경로> [--out <hw_sourcemap.json>]
//
// --out 을 생략하면 현재 디렉터리에 hw_sourcemap.json 으로 저장한다.

const fs = require('fs');
const path = require('path');
const { generate } = require('./index');

function usage() {
  console.log(
    [
      'usage: entryjs-sourcemap --src <entryjs 경로> [--out <hw_sourcemap.json>]',
      '',
      '옵션:',
      '  --src PATH   entryjs 체크아웃 경로 (필수)',
      '  --out FILE   출력 소스맵 경로 (기본: ./hw_sourcemap.json)',
      '  --help       도움말',
    ].join('\n')
  );
}

function parseArgs(argv) {
  const args = { src: null, out: 'hw_sourcemap.json' };
  for (let i = 0; i < argv.length; i++) {
    switch (argv[i]) {
      case '--src':
        args.src = argv[++i];
        break;
      case '--out':
        args.out = argv[++i];
        break;
      case '--help':
      case '-h':
        args.help = true;
        break;
      default:
        throw new Error(`unknown option: ${argv[i]}`);
    }
  }
  return args;
}

function runCli(argv) {
  let args;
  try {
    args = parseArgs(argv);
  } catch (e) {
    console.error(`error: ${e.message}`);
    usage();
    process.exit(1);
  }
  if (args.help) {
    usage();
    return;
  }
  if (!args.src) {
    console.error('error: --src 옵션이 필요합니다.');
    usage();
    process.exit(1);
  }
  if (!fs.existsSync(args.src)) {
    console.error(`error: entryjs 경로가 없습니다: ${args.src}`);
    process.exit(1);
  }

  const { sourcemap, stats } = generate(args.src);
  const outPath = path.resolve(args.out);
  fs.writeFileSync(outPath, JSON.stringify(sourcemap));
  console.log(`device count : ${stats.devices}`);
  console.log(`block total  : ${stats.blocks}`);
  console.log(`loaded       : ${stats.loaded} (failed: ${stats.failed})`);
  console.log(`written      : ${outPath}`);
}

module.exports = { runCli };
