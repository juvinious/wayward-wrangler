#!/usr/bin/env python3
"""Exercise an actual versioned production archive, installer and installed TUI.
Only the HTTPS download transport is local; no Waywr binary is substituted. All
processes, HOME/XDG paths and application state belong to this temporary fixture.
"""
import fcntl, hashlib, json, os, pathlib, pty, select, shutil, signal, socket as net, struct
import subprocess, sys, tempfile, termios, time
from installer_fixture import release_transport

ARCHIVE = pathlib.Path(sys.argv[1]).resolve()
SLOW_IPC = '--slow-ipc' in sys.argv[2:]
INSTALLER = pathlib.Path(__file__).with_name('install.sh').resolve()
def digest(path): return hashlib.sha256(path.read_bytes()).hexdigest()
def wait_for(check, reason, seconds=20):
    deadline = time.monotonic() + seconds
    while not check():
        if time.monotonic() > deadline: raise AssertionError(reason)
        time.sleep(.05)

def main():
    name = ARCHIVE.name.removesuffix('.tar.gz')
    version = name.removeprefix('waywr-').removesuffix('-linux-x86_64-gnu')
    assert version.startswith('v'), 'Use an explicitly versioned production build'
    with tempfile.TemporaryDirectory(prefix='wwpkg-') as tmp:
        root = pathlib.Path(tmp); home = root/'home'; home.mkdir()
        for p in ['config', 'data', 'state', 'cache', 'run', 'assets', 'transport', 'outside']:
            (root/p).mkdir(mode=0o700)
        assets=root/'assets'; shutil.copy2(ARCHIVE,assets/ARCHIVE.name)
        sums=assets/'SHA256SUMS'; sums.write_text(digest(ARCHIVE)+'  '+ARCHIVE.name+'\n')
        node=shutil.which('node'); assert node, 'Node runtime prerequisite missing'
        (root/'transport/node').symlink_to(node)
        fixture=release_transport(root/'transport', assets, version)
        env={'HOME':str(home),'PATH':str(root/'transport')+':/usr/bin:/bin','LANG':'C.UTF-8','TERM':'xterm-256color',
             'XDG_CONFIG_HOME':str(root/'config'),'XDG_DATA_HOME':str(root/'data'), 'XDG_STATE_HOME':str(root/'state'),
             'XDG_CACHE_HOME':str(root/'cache'),'XDG_RUNTIME_DIR':str(root/'run'),
             **fixture, 'DBUS_SESSION_BUS_ADDRESS':'unix:path='+str(root/'no-user-bus')}
        assert not shutil.which('cargo',path=env['PATH']) and not shutil.which('rustc',path=env['PATH'])
        trace=shutil.which('strace',path=env['PATH']); traces=[]
        assert not SLOW_IPC or trace, '--slow-ipc requires strace'
        def traced(args, slow=False):
            if not trace:return list(map(str,args))
            path=root/f'files-{len(traces)}.log';traces.append(path)
            delays=['--inject=read:delay_exit=100us','--inject=recvfrom:delay_exit=100us'] if slow else []
            calls='trace=%file,read,recvfrom,recvmsg' if slow else 'trace=%file'
            return [trace,'-f','-e',calls,'-o',str(path)]+delays+list(map(str,args))
        def run(args, ok=True, input=None):
            p=subprocess.run(traced(args),env=env,cwd=root/'outside',text=True,capture_output=True,timeout=30,input=input)
            assert (p.returncode==0)==ok, (args,p.stdout,p.stderr)
            return p
        installer=root/'install.sh'; shutil.copy2(INSTALLER,installer)
        # Exercise the curl | sh entry shape without credentials or gh access.
        result=run(['sh','-s','--','--repo','example/wayward-wrangler'], input=installer.read_text())
        assert 'export PATH=' in result.stdout and 'session upgrade' not in result.stdout
        prefix=home/'.local/share/wayward-wrangler'; installed=prefix/'releases'/version
        launcher=home/'.local/bin/waywr'
        assert launcher.resolve()==(installed/'bin/waywr').resolve()
        manifest=json.loads((installed/'manifest.json').read_text()); assert manifest['release_id']==version
        assert manifest['license']=='MIT' and manifest['notices']=='THIRD_PARTY_NOTICES.md'
        assert (installed/'LICENSE').is_file() and (installed/'THIRD_PARTY_NOTICES.md').is_file()
        # Manifest values are file metadata, not an executable replacement.
        for relative, item in manifest['files'].items():
            expected=item['sha256'] if isinstance(item,dict) else item
            assert digest(installed/relative)==expected, relative
        # Exact-version installation is also public and idempotent.
        run(['sh',installer,'--version',version,'--repo','example/wayward-wrangler'])
        selected=(prefix/'current').readlink()
        with (assets/ARCHIVE.name).open('ab') as damaged:
            damaged.write(b'CORRUPT DOWNLOAD')
        refused_download=run(['sh',installer,'--repo','example/wayward-wrangler'],ok=False)
        assert 'checksum mismatch' in refused_download.stderr
        assert (prefix/'current').readlink()==selected
        assert version in run([launcher,'--version']).stdout
        (assets/ARCHIVE.name).unlink()
        missing_download=run(['sh',installer,'--repo','example/wayward-wrangler'],ok=False)
        assert 'archive download failed' in missing_download.stderr
        assert (prefix/'current').readlink()==selected
        assert not list((prefix/'releases').glob('.install-*'))
        requests=(root/'transport/requests').read_text()
        assert '/releases/latest' in requests and '/releases/tags/'+version in requests
        info=run([launcher,'--backend-info']); assert 'rust' in info.stdout.lower()
        run([launcher,'--help']); run([launcher,'bundle-install','--help'])
        # Declaring notice metadata cannot bypass its required-file checks by
        # editing the self-contained manifest and removing a license entry.
        missing=root/'missing-license'; shutil.copytree(installed,missing)
        metadata=json.loads((missing/'manifest.json').read_text())
        metadata['files'].pop('LICENSE'); (missing/'LICENSE').unlink()
        (missing/'manifest.json').write_text(json.dumps(metadata))
        refused=root/'must-not-install-without-license'
        result=run([launcher,'bundle-install','--bundle',missing,'--prefix',refused],ok=False)
        assert 'license notices' in result.stderr and not refused.exists()
        shutil.rmtree(missing)
        # Reject a damaged real package before changing the isolated prefix.
        broken=root/'damaged'; shutil.copytree(installed,broken)
        resource=next(p for p in broken.rglob('*') if p.is_file() and 'resources' in p.parts)
        resource.write_bytes(resource.read_bytes()+b'\nCORRUPT')
        badprefix=root/'must-not-install'
        run([launcher,'bundle-install','--bundle',broken,'--prefix',badprefix],ok=False)
        assert not (badprefix/'releases'/version).exists()
        shutil.rmtree(broken); shutil.rmtree(assets); installer.unlink()
        # Startup uses only installed paired resources, outside the source tree.
        project=root/'project'; project.mkdir()
        init=json.loads(run([launcher,'init',project]).stdout)
        key=init['project_key']; state=pathlib.Path(init['state'])
        socket=root/'run/waywr'/f'{key}.sock'; socket.parent.mkdir(exist_ok=True)
        service_log=open(root/'service.log','wb'); service=None; ui=None; master=None
        try:
            service=subprocess.Popen(traced([str(installed/'bin/waywr-service'),'--project',str(project),'--state-dir',str(state),
                '--project-key',key,'--release-id',version,'--ipc',str(socket),'--runner',str(installed/'bin/waywr-runner'),
                '--frontend',str(installed/'bin/waywr')]),env=env,cwd=root/'outside',stdout=service_log,stderr=service_log,start_new_session=True)
            wait_for(lambda:socket.exists() or service.poll() is not None,'Service socket unavailable')
            assert service.poll() is None,(root/'service.log').read_text()
            # Exercise the actual public handshake, including the distinct IPC
            # and persistent-state versions (no command/approval is submitted).
            with net.socket(net.AF_UNIX,net.SOCK_STREAM) as peer:
                peer.connect(str(socket));peer.settimeout(5)
                peer.sendall((json.dumps({'v':1,'kind':'hello','request_id':'install-read','body':{
                    'protocol':{'major':1,'minor':10},'release_id':version,'project_key':key,
                    'client_id':'packaged-install','channel':'command'}})+'\n').encode())
                reply=json.loads(peer.makefile('rb').readline())
                assert reply['body']['accepted'] and reply['body']['state_schema']==9,reply
                assert reply['body']['protocol']['minor']==10,reply
            status=json.loads(run([launcher,'--project',project,'session','status']).stdout)
            master, slave=pty.openpty();fcntl.ioctl(slave,termios.TIOCSWINSZ,struct.pack('HHHH',32,120,0,0))
            ui=subprocess.Popen(traced([str(launcher),'--project',str(project)],slow=SLOW_IPC),env=env,cwd=root/'outside',stdin=slave,stdout=slave,stderr=slave,start_new_session=True);os.close(slave)
            screen=bytearray();deadline=time.monotonic()+20
            while b'Message the coordinator' not in screen and time.monotonic()<deadline:
                if select.select([master],[],[],.1)[0]:
                    try: screen.extend(os.read(master,65536))
                    except OSError: break
                assert ui.poll() is None,screen.decode(errors='replace')
            assert b'Message the coordinator' in screen,screen.decode(errors='replace')+'\nFixture service log:\n'+(root/'service.log').read_text(errors='replace')
            os.write(master,b'/detach\r');time.sleep(.2);os.write(master,b'\x1b[C\r')
            wait_for(lambda:ui.poll() is not None,'TUI did not detach')
            assert ui.returncode==0,screen.decode(errors='replace')
            # Read-only status after detach; no user request/approval was sent.
            final=json.loads(run([launcher,'--project',project,'session','status']).stdout)
            assert not list(state.rglob('ollama-send-intent.json'))
            assert not list(state.rglob('spawn-committed.json'))
            source=str(INSTALLER.parent.parent)
            assert not any(source in p.read_text(errors='replace') for p in traces), 'Installed runtime accessed source checkout'
            print(json.dumps({'archive':ARCHIVE.name,'archive_sha256':digest(ARCHIVE),'manifest_sha256':digest(installed/'manifest.json'),
                'release':version,'manifest_files':len(manifest['files']),'real_installer':True,'https_transport':'fixture, no GitHub download','github_cli_used':False,'latest_stable':True,'exact_version':True,'failed_download_preserves_selection':True,
                'installed_launcher':True,'real_service_and_tui':True,'rust_on_path':False,'slow_ipc_reads':SLOW_IPC,'runtime_file_trace':bool(trace),'source_file_accesses':0 if trace else 'not traced','cwd':'outside source checkout',
                'damaged_resource_refused':True,'native_calls':0,'scope':'idle startup and detach; no systemd bootstrap or ordinary development job'}))
        finally:
            if ui and ui.poll() is None: os.killpg(ui.pid,signal.SIGTERM);ui.wait(timeout=5)
            if master is not None: os.close(master)
            if service and service.poll() is None: os.killpg(service.pid,signal.SIGTERM);service.wait(timeout=10)
            service_log.close()

if __name__=='__main__':main()
