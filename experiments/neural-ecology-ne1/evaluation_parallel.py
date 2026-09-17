"""Independent CPU workers, bounded GPU collection, global fit/test barrier.

This scheduling wrapper is sealed separately before evaluation. It calls the
same per-seed primitives used by development, with no scientific parameter changes.
"""
from concurrent.futures import ProcessPoolExecutor
import json
import multiprocessing
import importlib.util
import measurement as m
import pipeline as p

# The inherited probe adds its own directory to sys.path. Load this experiment's
# driver explicitly so an old experiment's run.py cannot shadow it.
_spec=importlib.util.spec_from_file_location('ne1_driver',m.BASE/'run.py')
run=importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(run)

GPU_LOCK=None


def initialize_worker(lock):
    global GPU_LOCK
    GPU_LOCK=lock


def fit_seed(seed):
    sealpath=m.BASE/'evaluation-seal.json'
    p.verify_code(sealpath)
    with GPU_LOCK:
        p.prepare(seed)
        for split in ('fit','validation'):p.collect(seed,split,sealpath)
    p.fit_neural(seed)
    p.fit_calibration(seed)
    return seed


def test_seed(seed):
    sealpath=m.BASE/'evaluation-seal.json'
    p.verify_code(sealpath)
    with GPU_LOCK:
        p.collect(seed,'test',sealpath)
    p.score_seed(seed)
    return seed


def main():
    sealpath=m.BASE/'evaluation-seal.json'
    run.committed(sealpath)
    seal=p.verify_code(sealpath)
    assert seal['stage']=='evaluation'
    assert str((m.BASE/'evaluation_parallel.py').relative_to(m.REPO)) in seal['files']
    development=p.read(m.BASE/'development-results.json')
    assert development['complete'] and development['integrity_pass'] and development['calibration_pass']
    seeds=list(range(4001,4011))
    context=multiprocessing.get_context('spawn')
    lock=context.BoundedSemaphore(4)
    print(json.dumps(dict(stage='evaluation-fit',cpu_workers=10,gpu_workers=4,seeds=seeds)),flush=True)
    with ProcessPoolExecutor(max_workers=10,mp_context=context,initializer=initialize_worker,initargs=(lock,)) as pool:
        assert list(pool.map(fit_seed,seeds))==seeds
    run.fit_manifest(seeds,sealpath)
    print(json.dumps(dict(stage='evaluation-test',fit_manifest_sha256=m.sha(p.ART/'evaluation_fit_manifest.json'))),flush=True)
    with ProcessPoolExecutor(max_workers=10,mp_context=context,initializer=initialize_worker,initargs=(lock,)) as pool:
        assert list(pool.map(test_seed,seeds))==seeds
    run.audit(seeds,sealpath)


if __name__=='__main__':main()
