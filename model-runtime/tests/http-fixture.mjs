import {createServer} from 'node:http';
import assert from 'node:assert/strict';
export async function fixture(api) {
  let calls=0;
  const requests=[];
  const server=createServer(async(req,res)=>{
    let raw='';for await(const c of req)raw+=c;
    const input=JSON.parse(raw);requests.push(input);assert.equal(input.model,'model-outside-core');
    if(api==='openai-completions')assert.equal(input.temperature,0.2);
    const tools=input.tools||[];
    const properties=tools.find(t=>(t.name||t.function?.name)==='submit_result');
    const schema=properties?.input_schema||properties?.function?.parameters||{};
    const coordinator=!!schema.properties?.assistant_text;
    const first=calls++===0;
    const name=first&&!coordinator?'write':'submit_result';
    let args=first?{path:'delivered.txt',content:'Created by the actual SDK tool loop'}:
      {schema_version:1,summary:'HTTP fixture result',commit:'',checks:['fixture tool loop'],remaining_obligations:['No real model was called']};
    if(coordinator){
      assert.equal(tools.length,1,'Coordinator fixture must not receive worker tools');
      args=schema.properties.evidence_refs?{schema_version:3,assistant_text:'HTTP fixture coordinator reply',evidence_refs:[],action:null}:
        {schema_version:1,assistant_text:'HTTP fixture coordinator reply',action:'wait',tasks:[],units:[],checks:[],resolved_obligations:[],reason:'Fixture only'};
    }
    res.writeHead(200,{'content-type':'text/event-stream'});
    if(api==='openai-completions'){
      const data=v=>res.write('data: '+JSON.stringify(v)+'\n\n');
      data({id:'fixture',object:'chat.completion.chunk',created:1,model:input.model,choices:[{index:0,delta:{role:'assistant',tool_calls:[{index:0,id:'call-'+calls,type:'function',function:{name,arguments:JSON.stringify(args)}}]},finish_reason:null}]});
      data({id:'fixture',object:'chat.completion.chunk',choices:[{index:0,delta:{},finish_reason:'tool_calls'}],usage:{prompt_tokens:20,completion_tokens:8,prompt_tokens_details:{cached_tokens:2}}});
      res.end('data: [DONE]\n\n');
    }else{
      const event=(name,data)=>res.write('event: '+name+'\ndata: '+JSON.stringify({type:name,...data})+'\n\n');
      event('message_start',{message:{id:'fixture',type:'message',role:'assistant',content:[],model:input.model,stop_reason:null,usage:{input_tokens:20,output_tokens:0,cache_read_input_tokens:2,cache_creation_input_tokens:0}}});
      event('content_block_start',{index:0,content_block:{type:'tool_use',id:'call-'+calls,name,input:{}}});
      event('content_block_delta',{index:0,delta:{type:'input_json_delta',partial_json:JSON.stringify(args)}});
      event('content_block_stop',{index:0});event('message_delta',{delta:{stop_reason:'tool_use'},usage:{output_tokens:8}});event('message_stop',{});res.end();
    }
  });
  await new Promise(done=>server.listen(0,'127.0.0.1',done));
  return {endpoint:`http://127.0.0.1:${server.address().port}/v1`,calls:()=>calls,requests:()=>requests,close:()=>new Promise(done=>server.close(done))};
}

if(process.argv[2]){const server=await fixture(process.argv[2]);console.log(server.endpoint);process.stdin.resume();process.stdin.on('end',()=>server.close());}
