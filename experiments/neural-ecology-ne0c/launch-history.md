# NE-0c pre-evaluation launch history

- The first fixture attempt could not access a Metal device in the sandbox.
  Approved GPU access allowed all 700 fixtures to pass. No neural candidate
  executed during the failed fixture attempt.
- After full seed-3001 validation, the first seal command was blocked by the
  sandbox's restriction on `sysctl machdep.cpu.brand_string`. Consequently no seal
  file was written. The subsequent staging/commit commands failed, and the
  evaluation entry point rejected the missing seal before constructing a network
  or generating an evaluation stream. No `eval-*` artifacts existed.
- The unchanged seal command succeeded with approved metadata access. The seal
  was committed before the actual evaluation launch. No scientific parameter,
  implementation source, probe, seed, or threshold changed during this recovery.
