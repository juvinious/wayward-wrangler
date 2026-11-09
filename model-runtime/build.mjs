import {build} from 'esbuild';
import {mkdir,readFile,writeFile,readdir,stat} from 'node:fs/promises';
import {join} from 'node:path';
await mkdir('dist',{recursive:true});
await build({entryPoints:['main.mjs'],outfile:'dist/bridge.mjs',bundle:true,platform:'node',format:'esm',target:'node22',
  legalComments:'external',banner:{js:"import { createRequire as waywrCreateRequire } from 'node:module'; const require = waywrCreateRequire(import.meta.url);"}});
const lock=JSON.parse(await readFile('package-lock.json','utf8'));
let text='# Model runtime third-party notices\n\nPinned npm libraries used to build the shared provider/tool runtime.\n';
for(const [path,record] of Object.entries(lock.packages)) {
  if(!path||record.dev||record.optional)continue;
  const pkg=JSON.parse(await readFile(join(path,'package.json'),'utf8'));
  text+=`\n## ${pkg.name} ${pkg.version}\n\nLicense: ${pkg.license||'see upstream'}\n`;
  const files=(await readdir(path)).filter(n=>/^(license|licence|notice|copying)/i.test(n));
  if(!files.length&&pkg.name.startsWith('@earendil-works/')){
    text+='\n```text\n'+(await readFile('licenses/pi-MIT.txt','utf8')).trim()+'\n```\n';
    continue;
  }
  if(!files.length&&pkg.license==='Apache-2.0'){
    text+='\n```text\n'+(await readFile('node_modules/@aws-sdk/client-bedrock-runtime/LICENSE','utf8')).trim()+'\n```\n';
    continue;
  }
  if(!files.length&&pkg.license==='MIT'){
    text+='\nPublished package omits its license file. Attribution from its package metadata: '+JSON.stringify(pkg.author||pkg.contributors||pkg.name)+'.\n';
    const mit=await readFile('licenses/pi-MIT.txt','utf8');
    text+='\n```text\n'+mit.slice(mit.indexOf('Permission is hereby granted')).trim()+'\n```\n';
    continue;
  }
  if(!files.length)throw Error(`Missing license text for ${pkg.name}`);
  const include=async file=>{if((await stat(file)).isDirectory()){for(const name of await readdir(file))await include(join(file,name));}
    else text+='\n```text\n'+(await readFile(file,'utf8')).trim()+'\n```\n';};
  for(const file of files)await include(join(path,file));
}
await writeFile('dist/THIRD_PARTY_NOTICES.md',text);
