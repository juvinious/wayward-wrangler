import {builtinModels} from '@earendil-works/pi-ai/providers/all';
import {createProvider,getSupportedThinkingLevels,clampThinkingLevel} from '@earendil-works/pi-ai';
import {openAICompletionsApi} from '@earendil-works/pi-ai/api/openai-completions.lazy';
import {openAIResponsesApi} from '@earendil-works/pi-ai/api/openai-responses.lazy';
import {anthropicMessagesApi} from '@earendil-works/pi-ai/api/anthropic-messages.lazy';
import {googleGenerativeAIApi} from '@earendil-works/pi-ai/api/google-generative-ai.lazy';
import {Agent} from '@earendil-works/pi-agent-core';
import {readFile,writeFile,mkdir,rename} from 'node:fs/promises';
import {dirname,join} from 'node:path';
import {createInterface} from 'node:readline/promises';
import {spawn} from 'node:child_process';
import {once} from 'node:events';
import {Writable} from 'node:stream';
import {taskTools} from './tools.mjs';
import {liveInput} from './live-input.mjs';

const emit=value=>process.stdout.write(JSON.stringify(value)+'\n');
const protocols={'openai-completions':openAICompletionsApi,'openai-responses':openAIResponsesApi,
  'anthropic-messages':anthropicMessagesApi,'google-generative-ai':googleGenerativeAIApi};
const forbidden=new Set(['apiKey','headers','baseUrl','signal','fetch','onPayload','onResponse','onProviderStreamEvent','identity']);
async function loadConfig(path) {
  if(!path)return {};
  try{return JSON.parse(await readFile(path,'utf8'));}catch{throw Error('Model configuration is unreadable or invalid JSON');}
}

// The SDK owns API-key/OAuth semantics. Keep credentials out of project packets.
function credentialStore(path,enabled) {
  const read=async()=>{if(!enabled)return {};try{return JSON.parse(await readFile(path,'utf8'));}catch(e){if(e.code==='ENOENT')return {};throw Error('Private credential store is unreadable or invalid; preserve and repair it');}};
  return {
    read:async id=>(await read())[id],
    list:async()=>Object.entries(await read()).map(([providerId,c])=>({providerId,type:c.type})),
    modify:async(id,fn)=>{
      if(!enabled)throw Error('Credential writes unavailable during catalog inspection');
      await mkdir(dirname(path),{recursive:true,mode:0o700});
      const locker=spawn('flock',['--exclusive','--timeout','15',path+'.lock','/bin/sh','-c','printf ready; cat >/dev/null'],{stdio:['pipe','pipe','ignore']});
      await new Promise((done,fail)=>{locker.once('error',fail);locker.stdout.once('data',done);locker.once('exit',code=>{if(code)fail(Error('Credential store is busy'));});});
      try {const all=await read(),next=await fn(all[id]);if(next===undefined)return all[id];all[id]=next;
        const temp=path+'.'+process.pid+'.tmp';await writeFile(temp,JSON.stringify(all),{mode:0o600,flag:'wx'});await rename(temp,path);return next;
      }finally{locker.stdin.end();await once(locker,'close');}
    },
    delete:async id=>{throw Error('Use explicit credential management to remove credentials');},
  };
}
function authPath(){return join(process.env.XDG_CONFIG_HOME||join(process.env.HOME||'.','.config'),'waywr','auth.json');}
async function registry(config,authenticated=false) {
  const models=builtinModels({credentials:credentialStore(authPath(),authenticated),identity:{attribution:null}});
  for(const p of config.providers||[]) {
    const api=protocols[p.api];if(!api)throw Error(`Unknown protocol ${p.api}; install a runtime supporting it`);
    if(!p.id||!p.base_url)throw Error('Custom providers require id, api and base_url');
    const auth={apiKey:{name:p.auth_env||'Local endpoint',resolve:async()=>{
      const key=p.auth_env?process.env[p.auth_env]:undefined;
      if(p.auth_env&&!key)return undefined;
      return {auth:{apiKey:key||'local-endpoint'},source:p.auth_env||'keyless endpoint'};
    }}};
    models.setProvider(createProvider({id:p.id,name:p.name||p.id,baseUrl:p.base_url,auth,
      api:{[p.api]:api()},models:(p.models||[]).map(m=>({id:m.id,name:m.name||m.id,provider:p.id,api:p.api,
        baseUrl:p.base_url,reasoning:m.reasoning??false,thinkingLevelMap:m.thinking_level_map,input:['text'],contextWindow:m.context_window??0,
        maxTokens:m.max_tokens??4096,cost:m.cost||{input:0,output:0,cacheRead:0,cacheWrite:0}}))}));
  }
  return models;
}
async function describe(input) {
  const config=await loadConfig(input.config),models=await registry(config);
  const list=models.getModels().map(m=>({provider:m.provider,model:m.id,name:m.name,
    context_window:m.contextWindow||null,reasoning:m.reasoning,
    thinking_levels:getSupportedThinkingLevels(m),thinking_default:clampThinkingLevel(m,'off'),source:'runtime catalog/configuration',
    authentication:'not checked',capabilities:['conversation','coordinator','implementation','review']}));
  const first=config.defaults||{provider:list[0]?.provider,model:list[0]?.model};
  emit({protocol:1,catalog_version:2,providers:models.getProviders().map(p=>({id:p.id,name:p.name,authentication:'not checked',
    methods:Object.entries(p.auth).filter(([,v])=>v).map(([k])=>k)})),models:list,default:first,
    configured_providers:(config.providers||[]).map(p=>p.id),inference:false});
}
async function run(input) {
  const p=input.packet,profile=p.execution;
  let submitted,sent=false,responseComplete=false,usage={input:0,output:0,cacheRead:0,cacheWrite:0,cost:null,context_tokens:null,context_window:null};
  const runtime={provider:p.provider,requested_model:p.model,reported_model:null,
    model_identity:'SDK route; provider canonical identity not independently verified',usage,cost:null};
  let agent;
  const inbox=liveInput(input.directory,input.control_scope,emit);
  const cancel=()=>agent?.abort();process.on('SIGTERM',cancel);process.on('SIGINT',cancel);
  try {
    const config=await loadConfig(input.config||profile.runtime.config),models=await registry(config,true);
    const provider=models.getProvider(p.provider);if(!provider)throw Error(`Provider ${p.provider} is not registered; configure it in the runtime models file`);
    let model=models.getModel(p.provider,p.model);const knownModel=!!model;
    if(!model){const template=provider.getModels()[0];if(!template)throw Error('Manual model requires provider protocol metadata in the runtime configuration');model={...template,id:p.model,name:p.model,contextWindow:0};}
    const options=profile.options||{};for(const key of Object.keys(options))if(forbidden.has(key))throw Error(`Option ${key} belongs to provider configuration, not model options`);
    const thinking=options.reasoning??clampThinkingLevel(model,'off');
    if(typeof thinking!=='string'||(knownModel&&!getSupportedThinkingLevels(model).includes(thinking)))throw Error('Selected thinking level is unsupported for this model; choose one of its runtime-advertised levels');
    runtime.requested_thinking_level=thinking;runtime.thinking_capabilities=knownModel?'runtime metadata':'unverified manual model';
    const {reasoning:requestedReasoning,...requestOptions}=options;
    if(!await models.getAuth(model))throw Error(`Provider ${p.provider} needs credentials; use waywr models login or its SDK environment variable`);
    const submit={name:'submit_result',label:'Submit result',description:'Finish this assigned operation with the exact required structured result.',parameters:input.schema,
      execute:async(_id,value)=>{if(submitted!==undefined)throw Error('Result already submitted');submitted=value;return {content:[{type:'text',text:'Result recorded for application validation'}],details:{},terminate:true};}};
    const worker=p.operation_kind==='development-worker';
    agent=new Agent({initialState:{model,thinkingLevel:thinking,systemPrompt:input.instructions+'\nFinish by calling submit_result; prose alone does not complete this operation.',
      tools:[...(worker?taskTools(p.worktree):[]),submit]},toolExecution:'sequential',identity:{attribution:null},
      streamFn:(model,context,base)=>{sent=true;return models.streamSimple(model,context,{...base,...requestOptions,signal:base.signal});},
      finishTurn:async()=>{for(const message of await inbox.claim())agent.steer(message);}});
    agent.subscribe(async e=>{
      if(e.type==='message_start'&&await inbox.appliedMessage(e.message))submitted=undefined;
      if(e.type==='turn_start')emit({type:'runtime.activity',id:'model',label:'Requesting model response',status:'running'});
      if(e.type==='message_update'&&e.assistantMessageEvent.type==='text_delta')emit({type:'runtime.text',text:e.assistantMessageEvent.delta});
      if(e.type==='tool_execution_start'||e.type==='tool_execution_end')emit({type:'runtime.activity',id:e.toolCallId,
        label:e.toolName==='submit_result'?'Submitting result':e.toolName,status:e.type==='tool_execution_start'?'running':'completed'});
      if(e.type==='message_end'&&e.message.role==='assistant'){
        const u=e.message.usage;
        if(u){for(const k of ['input','output','cacheRead','cacheWrite'])usage[k]+=u[k]||0;
          const cost=u.cost?.total;if(typeof cost==='number'&&cost>0)usage.cost=(usage.cost||0)+cost;
          usage.context_tokens=(u.input||0)+(u.output||0)+(u.cacheRead||0)+(u.cacheWrite||0);usage.context_window=model.contextWindow||null;
          emit({type:'runtime.usage',scope:'attempt',cumulative:true,usage:{...usage,cost_source:'provider or SDK catalog estimate'}});}
      }
    });
    emit({type:'runtime.started',provider:p.provider,model:p.model,thinking_level:thinking});
    await agent.prompt(JSON.stringify(p));
    const last=agent.state.messages.filter(m=>m.role==='assistant').at(-1);
    responseComplete=!!last&&!['error','aborted','pending'].includes(last.stopReason);
    if(last?.stopReason==='error'||last?.stopReason==='aborted')throw Error(last.errorMessage||last.stopReason);
    if(submitted===undefined)throw Error('Model finished without a structured result');
    runtime.cost=usage.cost;
    runtime.steering_inputs=inbox.applied;
    emit({type:'runtime.completed',outcome:'completed',provider:p.provider,model:p.model,result:submitted,runtime,request_sent:sent});
  } catch(error) {
    // Do not publish provider response bodies or resolved credentials as diagnostics.
    const reason=sent?'Model request did not produce a valid result; inspect provider setup or retry as new work':String(error.message).slice(0,500);
    emit({type:'runtime.completed',outcome:'failed',provider:p.provider,model:p.model,request_sent:sent,
      category:sent?'provider':'configuration',response_complete:responseComplete,reason,runtime});process.exitCode=3;
  } finally {process.off('SIGTERM',cancel);process.off('SIGINT',cancel);}
}
async function login(provider,type='oauth') {
  let muted=false;
  const output=new Writable({write(chunk,_encoding,done){if(!muted)process.stderr.write(chunk);done();}});output.isTTY=process.stderr.isTTY;
  const models=await registry({},true),rl=createInterface({input:process.stdin,output,terminal:!!process.stdin.isTTY});
  try{await models.login(provider,type,{prompt:async p=>{process.stderr.write(p.message+' ');muted=p.type==='secret';try{return await rl.question('');}finally{muted=false;process.stderr.write('\n');}},notify:e=>{
    if(e.url)process.stderr.write(e.url+'\n');else if(e.verificationUri)process.stderr.write(e.verificationUri+' '+e.userCode+'\n');else if(e.message)process.stderr.write(e.message+'\n');}});
    process.stderr.write('Credentials saved in the private Waywr runtime store.\n');}finally{rl.close();}
}
try {
  const action=process.argv[2];
  if(action==='login')await login(process.argv[3],process.argv[4]);
  else {let text='';for await(const chunk of process.stdin){text+=chunk;if(text.length>8*1024*1024)throw Error('Runtime input too large');}
    const input=JSON.parse(text||'{}');if(action==='describe')await describe(input);else if(action==='run')await run(input);else throw Error('Expected describe, run or login');}
}catch(error){process.stderr.write(String(error.message).slice(0,500)+'\n');process.exitCode=3;}
