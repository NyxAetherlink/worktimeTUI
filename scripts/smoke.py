import os, pty, subprocess, select, time, fcntl, termios, struct, json, tempfile
binary=os.path.abspath(os.path.join(os.path.dirname(__file__), '../target/debug/worktimeTUI'))
def launch(directory):
    master,slave=pty.openpty()
    fcntl.ioctl(slave,termios.TIOCSWINSZ,struct.pack('HHHH',32,110,0,0))
    proc=subprocess.Popen([binary,'--data-dir',directory],stdin=slave,stdout=slave,stderr=slave,env={**os.environ,'TERM':'xterm-256color'})
    os.close(slave)
    return proc,master
def drain(fd,seconds=.3):
    end=time.monotonic()+seconds
    result=b''
    while time.monotonic()<end:
        if select.select([fd],[],[],.05)[0]:
            try: result+=os.read(fd,65536)
            except OSError: break
    return result
def send(fd,value,seconds=.3):
    os.write(fd,value);return drain(fd,seconds)
with tempfile.TemporaryDirectory(prefix='worktime-smoke-') as d:
    p,fd=launch(d)
    screen=drain(fd)
    assert b'worktimeTUI' in screen
    send(fd,b'nSmoke project\r')
    send(fd,b' ',1.3)
    send(fd,b' ')
    data=json.load(open(d+'/projects.json'))
    initial=data['projects'][0]['focus_ms']
    assert 1000 <= initial < 2500, data
    drain(fd,.5)
    send(fd,b'q'); assert p.wait(timeout=3)==0
    os.close(fd)
    data=json.load(open(d+'/projects.json'))
    assert data['projects'][0]['focus_ms']==initial
    p,fd=launch(d); screen=drain(fd)
    assert b'Smoke project' in screen and b'PAUSED' in screen
    # SGR mouse click inside the START button (x=60, y=15 at 110x32).
    send(fd,b'\x1b[<0;60;15M\x1b[<0;60;15m',1.2)
    send(fd,b' ')
    data=json.load(open(d+'/projects.json'))
    assert data['projects'][0]['focus_ms'] > initial + 900, data
    send(fd,b'pb')
    send(fd,b'nSecond\r')
    send(fd,b'q'); assert p.wait(timeout=3)==0
    os.close(fd)
    data=json.load(open(d+'/projects.json'))
    assert len(data['projects'])==2 and data['pomodoro'] and data['include_breaks']
    print('PASS: real PTY create, start, pause, mouse start, save, reopen paused, mode, break preference, second project')
