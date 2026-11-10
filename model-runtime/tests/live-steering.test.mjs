// Actual SDK tool loop; the endpoint and all files are local fixtures.
import {test} from 'node:test';
import assert from 'node:assert/strict';
import {createServer} from 'node:http';
import {spawn} from 'node:child_process';
import {mkdtemp,mkdir,writeFile,readFile,rm} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import {join,resolve} from 'node:path';

for(const api of ['openai-completions','anthropic-messages'])test(`live steering reaches ${api} before its next tool step`, {timeout:15000}, async()=>{
  const root=await mkdtemp(join(tmpdir(),'waywr-live-input-'));
  const scope={project_key:'a'.repeat(32),turn_id:'fixture-turn',attempt_id:'b'.repeat(32),packet_sha256:'c'.repeat(64)};
  const requests=[];
  const server=createServer(async(req,res)=>{
    let raw='';for await(const chunk of req)raw+=chunk;
    const input=JSON.parse(raw);requests.push(input);
    if(requests.length<=2){
      await mkdir(join(root,'live-input/pending'),{recursive:true});
      await writeFile(join(root,'live-input/pending/input-1.json'),JSON.stringify({schema_version:1,input_id:'input-1',scope,text:'Use the revised fixture constraint',kind:'steer'}));
    }
    const name=requests.length===1?'write':'submit_result';
    const args=name==='write'?{path:'delivered.txt',content:'SDK tool fixture'}:
      {schema_version:1,summary:'Steered result',commit:'',checks:[],remaining_obligations:[]};
    res.writeHead(200,{'content-type':'text/event-stream'});
    if(api==='openai-completions'){
      const data=v=>res.write('data: '+JSON.stringify(v)+'\n\n');
      data({id:'fixture',object:'chat.completion.chunk',created:1,model:input.model,choices:[{index:0,delta:{role:'assistant',tool_calls:[{index:0,id:'tool-'+requests.length,type:'function',function:{name,arguments:JSON.stringify(args)}}]},finish_reason:null}]});
      data({id:'fixture',object:'chat.completion.chunk',choices:[{index:0,delta:{},finish_reason:'tool_calls'}]});res.end('data: [DONE]\n\n');
    }else{
      const event=(type,body)=>res.write('event: '+type+'\ndata: '+JSON.stringify({type,...body})+'\n\n');
      event('message_start',{message:{id:'fixture',type:'message',role:'assistant',content:[],model:input.model,stop_reason:null,usage:{input_tokens:10,output_tokens:0}}});
      event('content_block_start',{index:0,content_block:{type:'tool_use',id:'tool-'+requests.length,name,input:{}}});
      event('content_block_delta',{index:0,delta:{type:'input_json_delta',partial_json:JSON.stringify(args)}});
      event('content_block_stop',{index:0});event('message_delta',{delta:{stop_reason:'tool_use'},usage:{output_tokens:8}});event('message_stop',{});res.end();
    }
  });
  await new Promise(done=>server.listen(0,'127.0.0.1',done));
  let child;
  try{
    const config=join(root,'models.json');
    await writeFile(config,JSON.stringify({providers:[{id:'live-fixture',api,base_url:`http://127.0.0.1:${server.address().port}/v1`,models:[{id:'any-model'}]}]}));
    const schema={type:'object',properties:{schema_version:{type:'integer'},summary:{type:'string'},commit:{type:'string'},checks:{type:'array',items:{type:'string'}},remaining_obligations:{type:'array',items:{type:'string'}}},required:['schema_version','summary','commit','checks','remaining_obligations'],additionalProperties:false};
    child=spawn(process.execPath,[resolve('dist/bridge.mjs'),'run'],{env:{PATH:process.env.PATH,HOME:root},stdio:['pipe','pipe','pipe']});
    let output='',errors='';child.stdout.on('data',c=>output+=c);child.stderr.on('data',c=>errors+=c);
    child.stdin.end(JSON.stringify({protocol:1,directory:root,control_scope:scope,instructions:'Local fixture only',schema,packet:{operation_kind:'development-worker',provider:'live-fixture',model:'any-model',worktree:root,execution:{runtime:{config},options:{}}}}));
    assert.equal(await new Promise(done=>child.on('close',done)),0,errors+'\n'+output);
    assert.equal(requests.length,2);
    assert.match(JSON.stringify(requests[1].messages),/Use the revised fixture constraint/,'Guidance never reached the active SDK');
    const applied=JSON.parse(await readFile(join(root,'live-input/applied/input-1.json'),'utf8'));
    assert.equal(applied.input_id,'input-1');
    assert.deepEqual(output.trim().split('\n').map(JSON.parse).at(-1).runtime.steering_inputs,['input-1'],'Republished guidance must not be consumed twice');
  }finally{
    child?.kill('SIGKILL');server.closeAllConnections();await new Promise(done=>server.close(done));
    await rm(root,{recursive:true,force:true});
  }
});
