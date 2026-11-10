// Real provider SDKs and agent loop against HTTP fixtures, never provider accounts.
import {test} from 'node:test';
import assert from 'node:assert/strict';
import {createServer} from 'node:http';
import {spawn} from 'node:child_process';
import {mkdtemp,writeFile,readFile,rm} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import {join,resolve} from 'node:path';

import {fixture} from './http-fixture.mjs';
const workerSchema={type:'object',properties:{schema_version:{type:'integer'},summary:{type:'string'},commit:{type:'string'},checks:{type:'array',items:{type:'string'}},remaining_obligations:{type:'array',items:{type:'string'}}},required:['schema_version','summary','commit','checks','remaining_obligations'],additionalProperties:false};
test('catalog exposes per-model thinking choices and default from runtime capabilities',async()=>{
  const root=await mkdtemp(join(tmpdir(),'waywr-thinking-catalog-'));
  try {
    const config=join(root,'models.json');await writeFile(config,JSON.stringify({providers:[{id:'thinking-fixture',api:'openai-completions',base_url:'http://127.0.0.1:1',models:[
      {id:'reasoning-model',reasoning:true,thinking_level_map:{off:null,minimal:null,xhigh:'xhigh',max:null}},
      {id:'plain-model',reasoning:false}]}]}));
    const child=spawn(process.execPath,[resolve('dist/bridge.mjs'),'describe'],{env:{PATH:process.env.PATH,HOME:root},stdio:['pipe','pipe','pipe']});
    let text='';child.stdout.on('data',c=>text+=c);child.stderr.resume();child.stdin.end(JSON.stringify({config}));
    assert.equal(await new Promise(done=>child.on('close',done)),0);
    const data=JSON.parse(text),model=data.models.find(m=>m.provider==='thinking-fixture'&&m.model==='reasoning-model');
    assert.deepEqual(data.providers.find(p=>p.id==='anthropic').login_methods,['apiKey','oauth']);
    assert.deepEqual(data.providers.find(p=>p.id==='thinking-fixture').login_methods,[],'Environment-only providers must not advertise an interactive login');
    assert.deepEqual(model.thinking_levels,['low','medium','high','xhigh']);
    assert.equal(model.thinking_default,'low');
    assert.deepEqual(data.models.find(m=>m.provider==='thinking-fixture'&&m.model==='plain-model').thinking_levels,['off']);
  }finally{await rm(root,{recursive:true,force:true});}
});
for(const api of ['openai-completions','anthropic-messages'])test(`shared worker uses ${api} with tools, custom IDs and usage`,async()=>{
  const root=await mkdtemp(join(tmpdir(),'waywr-model-test-')),http=await fixture(api);
  try{
    const config=join(root,'models.json');await writeFile(config,JSON.stringify({providers:[{id:'custom-'+api,api,base_url:http.endpoint,models:[{id:'model-outside-core',context_window:65536,max_tokens:32768,reasoning:true}]}]}));
    const input={protocol:1,directory:root,instructions:'Fixture only',schema:workerSchema,packet:{operation_kind:'development-worker',provider:'custom-'+api,model:'model-outside-core',worktree:root,execution:{runtime:{config},options:{temperature:0.2,reasoning:'high'}}}};
    const child=spawn(process.execPath,[resolve('dist/bridge.mjs'),'run'],{env:{PATH:process.env.PATH,HOME:root},stdio:['pipe','pipe','pipe']});
    let stdout='',stderr='';child.stdout.on('data',c=>stdout+=c);child.stderr.on('data',c=>stderr+=c);child.stdin.end(JSON.stringify(input));
    const code=await new Promise(done=>child.on('close',done));
    assert.equal(code,0,stderr+'\n'+stdout);
    const events=stdout.trim().split('\n').map(JSON.parse);
    assert.equal(await readFile(join(root,'delivered.txt'),'utf8'),'Created by the actual SDK tool loop');
    assert.equal(http.calls(),2);
    for(const request of http.requests()){
      if(api==='openai-completions')assert.equal(request.reasoning_effort,'high');
      else {assert.equal(request.thinking.type,'enabled');assert(request.thinking.budget_tokens>0);}
    }
    assert(events.some(e=>e.type==='runtime.activity'&&e.label==='write'));
    assert(events.some(e=>e.type==='runtime.usage'&&e.usage.input>0));
    assert.equal(events.at(-1).outcome,'completed');assert.equal(events.at(-1).provider,'custom-'+api);
    assert.equal(events.at(-1).runtime.requested_thinking_level,'high');
  }finally{await http.close();await rm(root,{recursive:true,force:true});}
});

test('unsupported known-model thinking levels refuse before HTTP without silent downgrade',async()=>{
  const root=await mkdtemp(join(tmpdir(),'waywr-thinking-refusal-')),http=await fixture('openai-completions');
  try {
    const config=join(root,'models.json');await writeFile(config,JSON.stringify({providers:[{id:'thinking-fixture',api:'openai-completions',base_url:http.endpoint,
      models:[{id:'model-outside-core',reasoning:true,thinking_level_map:{xhigh:null,max:null}}]}]}));
    const child=spawn(process.execPath,[resolve('dist/bridge.mjs'),'run'],{env:{PATH:process.env.PATH,HOME:root},stdio:['pipe','pipe','pipe']});
    let text='';child.stdout.on('data',c=>text+=c);child.stderr.resume();
    child.stdin.end(JSON.stringify({packet:{provider:'thinking-fixture',model:'model-outside-core',execution:{runtime:{config},options:{reasoning:'xhigh'}}}}));
    assert.equal(await new Promise(done=>child.on('close',done)),3);
    const final=JSON.parse(text.trim().split('\n').at(-1));assert.equal(final.request_sent,false);assert.match(final.reason,/thinking level is unsupported/);assert.equal(http.calls(),0);
  }finally{await http.close();await rm(root,{recursive:true,force:true});}
});

test('concurrent API-key setup keeps both providers in one private store without account calls',async()=>{
  const root=await mkdtemp(join(tmpdir(),'waywr-auth-test-'));
  try {
    await Promise.all(['openai','anthropic'].map(provider=>new Promise((done,fail)=>{
      const child=spawn(process.execPath,[resolve('dist/bridge.mjs'),'login',provider,'apiKey'],{env:{PATH:process.env.PATH,HOME:root,XDG_CONFIG_HOME:root},stdio:['pipe','pipe','pipe']});
      let asked=false,stderr='';child.stderr.on('data',chunk=>{stderr+=chunk;if(!asked&&/key/i.test(stderr)){asked=true;child.stdin.end('fixture\n');}});
      child.on('error',fail);child.on('close',code=>code===0?done():fail(Error(stderr)));
    })));
    const data=JSON.parse(await readFile(join(root,'waywr/auth.json'),'utf8'));
    assert.deepEqual(Object.keys(data).sort(),['anthropic','openai']);
  }finally{await rm(root,{recursive:true,force:true});}
});

test('missing custom-provider credentials refuse before an HTTP request',async()=>{
  const root=await mkdtemp(join(tmpdir(),'waywr-no-request-')),http=await fixture('openai-completions');
  try {
    const config=join(root,'models.json');await writeFile(config,JSON.stringify({providers:[{id:'configured-endpoint',api:'openai-completions',
      base_url:http.endpoint,auth_env:'MISSING_FIXTURE_API_KEY',models:[{id:'model-outside-core'}]}]}));
    const child=spawn(process.execPath,[resolve('dist/bridge.mjs'),'run'],{env:{PATH:process.env.PATH,HOME:root},stdio:['pipe','pipe','pipe']});
    let text='';child.stdout.on('data',c=>text+=c);child.stderr.resume();
    child.stdin.end(JSON.stringify({packet:{provider:'configured-endpoint',model:'model-outside-core',execution:{runtime:{config},options:{}}}}));
    assert.equal(await new Promise(done=>child.on('close',done)),3);
    const final=JSON.parse(text.trim().split('\n').at(-1));assert.equal(final.request_sent,false);assert.equal(final.category,'configuration');assert.equal(http.calls(),0);
  }finally{await http.close();await rm(root,{recursive:true,force:true});}
});
