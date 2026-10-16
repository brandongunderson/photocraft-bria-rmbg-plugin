"""Exercise the exact extracted package. Pillow is test tooling, not a plugin dependency."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import queue
import shutil
import struct
import subprocess
import threading
import time
import zipfile
from PIL import Image

ROOT = Path(__file__).resolve().parents[1]

class Worker:
    def __init__(self, stage, work, plugin_id='com.photocraft.bria-rmbg'):
        self.log = (work / f'worker-{time.time_ns()}.log').open('wb')
        environment = os.environ.copy()
        environment['PATH'] = str(Path(os.environ['SystemRoot'])/'System32')
        self.process = subprocess.Popen([str(stage/'bria-rmbg-worker.exe'),'--serve'],cwd=stage,
            stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=self.log,env=environment)
        self.frames = queue.Queue()
        self.reader = threading.Thread(target=self.read,daemon=True)
        self.reader.start()
        self.next_id = 1
        self.send({'type':'hello','id':'0','protocolVersion':2,'pluginId':plugin_id,
            'generation':'test-generation','operations':['image'],'maxFrameBytes':262144})

    def read(self):
        try:
            while True:
                header = self.process.stdout.read(4)
                if len(header) != 4: break
                size, = struct.unpack('<I',header)
                if not 0 < size <= 262144: raise RuntimeError('Invalid worker frame size')
                self.frames.put(json.loads(self.process.stdout.read(size)))
        except Exception as error:
            self.frames.put(error)
        self.frames.put(EOFError('Worker exited'))

    def send(self, frame):
        data = json.dumps(frame).encode()
        self.process.stdin.write(struct.pack('<I',len(data))+data)
        self.process.stdin.flush()

    def receive(self, timeout=60):
        frame = self.frames.get(timeout=timeout)
        if isinstance(frame,Exception): raise frame
        return frame

    def start(self, request):
        correlation = str(self.next_id); self.next_id += 1
        self.send({'type':'run','id':correlation,'operation':'image','request':request})
        return correlation

    def finish(self, correlation, cancel_at=None):
        messages=[]; cancelled_at=None; started=time.monotonic()
        while True:
            frame=self.receive(120)
            assert frame['id']==correlation, frame
            if frame['type']=='progress':
                assert 0<=frame['fraction']<=1
                messages.append(frame['message'])
                if cancel_at is not None and frame['fraction']>=cancel_at and cancelled_at is None:
                    self.send({'type':'cancel','id':correlation}); cancelled_at=time.monotonic()
                continue
            return {'terminal':frame,'seconds':round(time.monotonic()-started,3),'messages':messages,
                'cancelSeconds':None if cancelled_at is None else round(time.monotonic()-cancelled_at,3)}

    def reset(self):
        correlation=str(self.next_id); self.next_id += 1
        self.send({'type':'reset','id':correlation})
        assert self.receive()=={'type':'resetComplete','id':correlation}

    def stop(self):
        if self.process.poll() is None:
            self.send({'type':'shutdown'})
            self.process.wait(timeout=10)
        self.log.close()

def main():
    parser=argparse.ArgumentParser()
    parser.add_argument('package',type=Path)
    parser.add_argument('--image',type=Path,required=True)
    parser.add_argument('--long-path',action='store_true',help='Reproduce an installed DLL path beyond Windows MAX_PATH')
    args=parser.parse_args()
    work=ROOT/'verification/work'/str(time.time_ns()); work.mkdir(parents=True)
    if args.long_path:
        suffix = len(str(Path('runtime/bin/onnxruntime.dll')))+1
        length = 262-suffix-len(str(work))-1
        if length < 1 or length > 255: raise RuntimeError('Cannot create regression path at this test root')
        stage=work/('x'*length)
    else:
        stage=work/'package'
    stage.mkdir()
    with zipfile.ZipFile(args.package) as archive: archive.extractall(stage)
    image=Image.open(args.image).convert('RGBA')
    image.thumbnail((640,640))
    for x in range(image.width): image.putpixel((x,0),(*image.getpixel((x,0))[:3],0))
    for x in range(image.width): image.putpixel((x,1),(*image.getpixel((x,1))[:3],128))
    image.save(work/'input.png')
    def job(name,output='foreground',**extra):
        result={'apiVersion':1,'input':str((work/'input.png').resolve()),'output':str((work/(name+'.png')).resolve()),
            'outputSize':{'width':image.width,'height':image.height},'colorSpace':'srgb','options':{'output':output}}
        result.update(extra); return result
    results={}
    worker=Worker(stage,work)
    try:
        assert worker.receive()['type']=='ready'
        for output in ['foreground','background','mask']:
            result=worker.finish(worker.start(job(output,output)))
            assert result['terminal']['type']=='result',result
            results[output]=result
            target=Image.open(work/(output+'.png')).convert('RGBA')
            assert target.size==image.size
            if output!='mask':
                assert target.convert('RGB').tobytes()==image.convert('RGB').tobytes()
                assert all(target.getpixel((x,0))[3]==0 for x in range(image.width))
                assert all(target.getpixel((x,1))[3]<=128 for x in range(image.width))
            else:
                assert all(pixel[0]==pixel[1]==pixel[2] and pixel[3]==255 for pixel in target.getdata())
        assert any('Loading' in message for message in results['foreground']['messages'])
        assert not any('Loading' in message for key in ['background','mask'] for message in results[key]['messages'])
        foreground=Image.open(work/'foreground.png').convert('RGBA')
        background=Image.open(work/'background.png').convert('RGBA')
        assert len(set(foreground.getchannel('A').getdata()))>32, 'Real model produced no useful edge variation'
        assert all(abs(f[3]+b[3]-s[3])<=1 for f,b,s in zip(foreground.getdata(),background.getdata(),image.getdata()))
        cancelled=worker.finish(worker.start(job('cancel')),cancel_at=0.30)
        assert cancelled['terminal']['type']=='cancelled',cancelled
        assert cancelled['cancelSeconds']<2,cancelled
        results['inferenceCancellation']=cancelled
        invalid=worker.finish(worker.start(job('invalid',options={'unsupported':True})))
        assert invalid['terminal']['type']=='error'
        results['invalidOptions']=invalid
        worker.reset()
        reloaded=worker.finish(worker.start(job('after-reset')))
        assert reloaded['terminal']['type']=='result'
        assert any('Loading' in message for message in reloaded['messages'])
        results['resetAndReload']=reloaded
        worker.reset()
        model=stage/'runtime/models/rmbg-1.4.onnx'; missing=model.with_suffix('.missing'); model.rename(missing)
        try:
            missing_result=worker.finish(worker.start(job('missing-model')))
            assert missing_result['terminal']['type']=='error'
            assert 'missing' in missing_result['terminal']['error']['message']
            results['missingModel']=missing_result
        finally: missing.rename(model)
    finally: worker.stop()
    bad=Worker(stage,work,plugin_id='other.plugin')
    bad.process.wait(timeout=5); assert bad.process.returncode!=0; bad.log.close()
    results['foreignHandshakeRejected']=True
    bad=Worker(stage,work)
    assert bad.receive()['type']=='ready'
    bad.process.stdin.write(struct.pack('<I',262145)); bad.process.stdin.flush()
    bad.process.wait(timeout=5); assert bad.process.returncode!=0; bad.log.close()
    results['oversizedFrameRejected']=True
    bad=Worker(stage,work)
    assert bad.receive()['type']=='ready'
    bad.start(job('eof'))
    bad.process.stdin.close()
    bad.process.wait(timeout=5); bad.log.close()
    results['eofStopsWorker']=True
    # Keep visual evidence outside the disposable extracted installation.
    for name in ['input','foreground','background','mask']:
        shutil.copy2(work/(name+'.png'),ROOT/'verification'/(name+'.png'))
    report={'passed':True,'package':str(args.package.resolve()),'sha256':hashlib.file_digest(args.package.open('rb'),'sha256').hexdigest(),
        'systemOnlyPath':True,'sourceImage':str(args.image.resolve()),'longPath':args.long_path,
        'dllPathCharacters':len(str(stage/'runtime/bin/onnxruntime.dll')),'results':results}
    report_name='package-long-path-smoke' if args.long_path else 'package-smoke'
    (ROOT/'verification'/(report_name+'.json')).write_text(json.dumps(report,indent=2)+'\n')
    (ROOT/'verification'/(report_name+'-diagnostics.log')).write_text('\n'.join(path.read_text(errors='replace') for path in work.glob('worker-*.log')))
    # Delete only this test's verified private child; retain evidence copied above.
    if work.resolve().parent != (ROOT/'verification/work').resolve():
        raise RuntimeError('Refusing cleanup outside the private test root')
    shutil.rmtree(work)
    print(json.dumps({'passed':True,'timings':{key:value['seconds'] for key,value in results.items() if isinstance(value,dict)},'work':str(work)},indent=2))

if __name__=='__main__': main()
