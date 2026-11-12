// Task tools for the library-owned agent loop. This is a trusted same-UID runner,
// not an OS sandbox. Waywr controls the assigned worktree and result adoption.
import {readFile,writeFile,mkdir} from 'node:fs/promises';
import {resolve,dirname} from 'node:path';
import {spawn} from 'node:child_process';

const schema=properties=>({type:'object',properties,required:Object.keys(properties),additionalProperties:false});
const str=description=>({type:'string',description});
const result=text=>({content:[{type:'text',text}],details:{}});
export function taskTools(cwd) {
  const file=name=>resolve(cwd,name);
  return [
    {name:'read',label:'Read file',description:'Read a UTF-8 file in the task checkout.',parameters:schema({path:str('File path')}),
      execute:async(_id,{path})=>{const bytes=await readFile(file(path));if(bytes.length>262144)throw Error('Read exceeds 256 KiB; use a bounded shell command');return result(bytes.toString('utf8'));}},
    {name:'write',label:'Write file',description:'Write UTF-8 content in the task checkout.',parameters:schema({path:str('File path'),content:str('Complete content')}),
      execute:async(_id,{path,content})=>{const target=file(path);await mkdir(dirname(target),{recursive:true});await writeFile(target,content);return result('File written');}},
    {name:'edit',label:'Edit file',description:'Replace one exact occurrence of text.',parameters:schema({path:str('File path'),old_text:str('Unique existing text'),new_text:str('Replacement text')}),
      execute:async(_id,{path,old_text,new_text})=>{const target=file(path),text=await readFile(target,'utf8');if(!old_text||text.split(old_text).length!==2)throw Error('Existing text must match exactly once');await writeFile(target,text.replace(old_text,new_text));return result('File edited');}},
    {name:'bash',label:'Run command',description:'Run a shell command from the assigned task checkout. No remote publication or unrelated project changes.',parameters:schema({command:str('Shell command')}),
      execute:async(_id,{command},signal,onUpdate)=>new Promise((done,fail)=>{
        const child=spawn('/bin/sh',['-c',command],{cwd,env:process.env,signal,stdio:['ignore','pipe','pipe']});
        let text='',error,bytes=0;
        const append=chunk=>{bytes+=chunk.length;if(bytes>262144){error=Error('Command output exceeded 256 KiB');child.kill('SIGTERM');return;}text+=chunk.toString();onUpdate?.(result(text));};
        child.stdout.on('data',append);child.stderr.on('data',append);
        child.on('error',e=>{error=e;});
        child.on('close',code=>error?fail(error):done({...result(text),details:{exitCode:code},isError:code!==0}));
      })},
  ];
}
