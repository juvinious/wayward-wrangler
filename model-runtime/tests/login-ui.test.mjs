// Real SDK API-key login, isolated private store; no provider request.
import {test} from 'node:test';
import assert from 'node:assert/strict';
import {spawn} from 'node:child_process';
import {createInterface} from 'node:readline';
import {mkdtemp,readFile,rm,stat} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import {join,resolve} from 'node:path';

for(const [cancel,method,promptType] of [[false,'apiKey','secret'],[true,'apiKey','secret'],[true,'oauth','select']])test(`in-app SDK ${method} login ${cancel?'cancels without saving':'saves privately without echoing secrets'}`,{timeout:10000},async()=>{
  const root=await mkdtemp(join(tmpdir(),'waywr-login-ui-'));
  const child=spawn(process.execPath,[resolve('dist/bridge.mjs'),'login-ui','anthropic',method],{env:{PATH:process.env.PATH,HOME:root,XDG_CONFIG_HOME:root},stdio:['pipe','pipe','pipe']});
  const deadline=setTimeout(()=>child.kill(),3000);
  let output='',errors='',prompt;
  child.stdout.on('data',c=>output+=c);child.stderr.on('data',c=>errors+=c);
  const lines=createInterface({input:child.stdout});
  lines.on('line',line=>{
    const event=JSON.parse(line);
    if(event.type==='auth.prompt'){
      prompt=event;child.stdin.write(JSON.stringify(cancel?{type:'cancel'}:{type:'answer',id:event.id,value:'fixture-private-key'})+'\n');
    }
  });
  try{
    const code=await new Promise(done=>child.on('close',done));
    assert(prompt,'No in-app authentication prompt was offered');assert.equal(prompt.prompt.type,promptType);
    assert(!output.includes('fixture-private-key'));assert(!errors.includes('fixture-private-key'));
    const final=JSON.parse(output.trim().split('\n').at(-1));assert.equal(final.type,'auth.completed');
    assert.equal(final.status,cancel?'cancelled':'authenticated');assert.equal(code,0,errors);
    const file=join(root,'waywr/auth.json');
    if(cancel)await assert.rejects(readFile(file));else{
      const data=JSON.parse(await readFile(file,'utf8'));assert.equal(data.anthropic.key,'fixture-private-key');assert.equal((await stat(file)).mode&0o777,0o600);
    }
  }finally{clearTimeout(deadline);child.kill();lines.close();await rm(root,{recursive:true,force:true});}
});

test('SDK browser callback cancels only its manual prompt and suppresses raw failure details',async()=>{
  const {PassThrough}=await import('node:stream');const {loginUI}=await import('../login-ui.mjs');
  const input=new PassThrough(),events=[];
  const models={login:async(_provider,_method,interaction)=>{
    interaction.notify({type:'auth_url',url:'https://example.invalid/authorize'});
    const callback=new AbortController();
    const manual=interaction.prompt({type:'manual_code',message:'Paste redirect if needed',signal:callback.signal});
    callback.abort();await assert.rejects(manual);
    throw Error('Token exchange response: fixture-private-key');
  }};
  await loginUI(models,'fixture-provider','oauth',{input,emit:e=>events.push(e)});
  assert(events.some(e=>e.type==='auth.prompt_cancelled'));
  assert(events.some(e=>e.type==='auth.event'&&e.event.type==='auth_url'));
  assert.equal(events.at(-1).status,'failed');assert(!JSON.stringify(events).includes('fixture-private-key'));
});

test('credential refresh failure cannot copy token-exchange response bodies into project diagnostics',async()=>{
  const {mkdir,writeFile}=await import('node:fs/promises');
  const root=await mkdtemp(join(tmpdir(),'waywr-refresh-private-'));
  try{
    await mkdir(join(root,'waywr'));
    await writeFile(join(root,'waywr/auth.json'),JSON.stringify({anthropic:{type:'oauth',access:'fixture-access',refresh:'fixture-refresh',expires:0}}));
    const preload=join(root,'fetch-fixture.mjs');
    await writeFile(preload,"globalThis.fetch=async()=>new Response('fixture-private-refresh-detail',{status:400});\n");
    const child=spawn(process.execPath,['--import',preload,resolve('dist/bridge.mjs'),'run'],{env:{PATH:process.env.PATH,HOME:root,XDG_CONFIG_HOME:root},stdio:['pipe','pipe','pipe']});
    let output='';child.stdout.on('data',c=>output+=c);child.stderr.resume();
    child.stdin.end(JSON.stringify({packet:{provider:'anthropic',model:'fixture-model',execution:{runtime:{},options:{}}}}));
    const code=await new Promise(done=>child.on('close',done));assert.equal(code,3);
    const result=JSON.parse(output.trim().split('\n').at(-1));assert.equal(result.request_sent,false);
    assert(!output.includes('fixture-private-refresh-detail'),'Private refresh response leaked into runtime diagnostics');
    assert.match(result.reason,/credentials|sign.in/i);
  }finally{await rm(root,{recursive:true,force:true});}
});
