// One attempt's private, service-owned inbox. Atomic rename decides whether an
// edit/withdrawal or the agent boundary claims a pending input first.
import {constants} from 'node:fs';
import {readdir,mkdir,rename,open,writeFile,unlink} from 'node:fs/promises';
import {join} from 'node:path';

export function liveInput(directory,scope,emit){
  const applied=[];
  const valid=id=>typeof id==='string'&&/^[a-zA-Z0-9_-]{1,120}$/.test(id);
  const root=directory&&scope?join(directory,'live-input'):null;
  const claimed=new Map();
  async function claim(){
    if(!root)return [];
    let names;
    try{names=await readdir(join(root,'pending'));}catch(e){if(e.code==='ENOENT')return [];throw e;}
    if(names.length>256)throw Error('Live input inbox exceeds its bound');
    await mkdir(join(root,'claimed'),{recursive:true,mode:0o700});
    const messages=[];
    for(const name of names.sort()){
      const id=name.endsWith('.json')?name.slice(0,-5):'';
      if(!valid(id))continue;
      if(applied.includes(id)||claimed.has(id)){
        try{await unlink(join(root,'pending',name));}catch(e){if(e.code!=='ENOENT')throw e;}
        continue;
      }
      const target=join(root,'claimed',name);
      try{await rename(join(root,'pending',name),target);}catch(e){if(e.code==='ENOENT')continue;throw e;}
      let input;
      try{
        const file=await open(target,constants.O_RDONLY|constants.O_NOFOLLOW);
        try{
          const stat=await file.stat();if(!stat.isFile()||stat.size>40960)throw Error('Invalid live input file');
          input=JSON.parse(await file.readFile('utf8'));
        }finally{await file.close();}
        if(input.schema_version!==1||input.input_id!==id||input.kind!=='steer'||typeof input.text!=='string'||!input.text.trim()||Buffer.byteLength(input.text)>32768||
          Object.keys(scope).length!==4||Object.keys(input.scope||{}).length!==4||
          !['project_key','turn_id','attempt_id','packet_sha256'].every(k=>input.scope[k]===scope[k]))throw Error('Invalid live input scope');
      }catch{
        emit({type:'runtime.input',input_id:id,status:'rejected',reason:'Invalid live input or attempt identity'});
        continue;
      }
      claimed.set(id,input);
      emit({type:'runtime.input',input_id:id,status:'delivering'});
      messages.push({role:'user',content:'Operator steering for this operation. Approved scope and tools are unchanged.\n'+input.text,timestamp:Date.now(),waywr_input_id:id});
    }
    return messages;
  }
  async function appliedMessage(message){
    const id=message?.waywr_input_id;if(!id||!claimed.has(id))return false;
    await mkdir(join(root,'applied'),{recursive:true,mode:0o700});
    const input=claimed.get(id);
    const temp=join(root,'applied',id+'.tmp');
    await writeFile(temp,JSON.stringify({...input,status:'applied'}),{mode:0o600});
    await rename(temp,join(root,'applied',id+'.json'));
    applied.push(id);claimed.delete(id);
    emit({type:'runtime.input',input_id:id,status:'applied'});
    return true;
  }
  return {claim,appliedMessage,applied};
}
