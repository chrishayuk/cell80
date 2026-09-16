# Implementation seal history

The scientific protocol remains v1.0, SHA-256
`f526a743eaefa4adf2a7f56721a5cfbcca3d4c502c64a54366b5c81c0eeb18d6`.

1. `implementation-seal.json`, committed at `1956385`, records the initial
   implementation and all development outcomes. **No evaluation data was generated
   under this seal.**
2. Before evaluation, code review found that concurrent processes unconditionally
   rewrote shared generated MSL/IR files. A seal verifier could observe a partial
   write even though all writers produced identical contents. The cache now skips
   identical files and atomically replaces changed files. Repeated fixtures leave
   those file modification times unchanged and preserve the original fixture hash.
   This changes file handling only; the neuron rule, plasticity rule, numerical
   parameters, grammar, probes, seed policy and statistical criteria are unchanged.
3. `implementation-seal-v2.json` seals that correction before evaluation. The
   original seal is retained. Use v2 for the ten evaluation seeds.
