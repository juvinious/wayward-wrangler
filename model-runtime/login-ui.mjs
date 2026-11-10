// Ephemeral authentication interaction. Credentials stay in the SDK-owned store;
// only prompts, browser handoffs and completion status cross the UI pipes.
import {createInterface} from 'node:readline';

export async function loginUI(models,provider,method,{input=process.stdin,emit}={}){
  const control=new AbortController(),waiting=new Map();let sequence=0;
  const cancel=()=>control.abort();
  const lines=createInterface({input,crlfDelay:Infinity});
  lines.on('line',line=>{
    if(Buffer.byteLength(line)>32768){cancel();return;}
    try{
      const reply=JSON.parse(line);
      if(reply.type==='cancel'){cancel();return;}
      const pending=waiting.get(reply.id);
      if(reply.type==='answer'&&pending&&typeof reply.value==='string'&&Buffer.byteLength(reply.value)<=4096)pending.resolve(reply.value);
    }catch{cancel();}
  });
  lines.on('close',cancel);process.on('SIGTERM',cancel);process.on('SIGINT',cancel);
  try{
    await models.login(provider,method,{
      signal:control.signal,
      prompt:p=>new Promise((resolve,reject)=>{
        const id=String(++sequence),signal=p.signal||control.signal;
        const abort=()=>{cleanup();emit({type:'auth.prompt_cancelled',id});reject(Error('Cancelled'));};
        const cleanup=()=>{waiting.delete(id);signal.removeEventListener('abort',abort);control.signal.removeEventListener('abort',abort);};
        waiting.set(id,{resolve:value=>{cleanup();resolve(value);}});
        signal.addEventListener('abort',abort,{once:true});if(signal!==control.signal)control.signal.addEventListener('abort',abort,{once:true});
        if(signal.aborted||control.signal.aborted){abort();return;}
        const prompt={type:p.type,message:p.message,placeholder:p.placeholder};
        if(p.type==='select')prompt.options=p.options.map(o=>({id:o.id,label:o.label,description:o.description}));
        emit({type:'auth.prompt',id,prompt});
      }),
      notify:event=>emit({type:'auth.event',event}),
    });
    emit({type:'auth.completed',status:'authenticated',provider});
  }catch{
    // SDK errors may contain token-exchange response bodies. Never forward them.
    emit({type:'auth.completed',status:control.signal.aborted?'cancelled':'failed',provider,
      message:control.signal.aborted?'Sign-in cancelled':'Sign-in did not complete. Check the provider/account setup and try again.'});
  }finally{
    lines.close();input.pause();process.off('SIGTERM',cancel);process.off('SIGINT',cancel);
  }
}
