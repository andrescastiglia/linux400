# *SRVPGM contract

Linux/400 treats `*SRVPGM` as a backlog object type for the current V1
toolchain. The object type is valid and can be cataloged, secured, copied, and
audited, but it is not a link target for `CALL`.

Required metadata for cataloged `*SRVPGM` objects:

- `user.l400.objtype=*SRVPGM`
- `user.l400.attr=SRVPGM`
- `user.l400.text`
- `user.l400.owner`
- `user.l400.public_auth`
- `user.l400.auth.manifest`

Future linking/loading metadata is reserved under:

- `user.l400.srvpgm.exports`
- `user.l400.srvpgm.imports`
- `user.l400.srvpgm.signature`

Authority follows object authority rules. Creating or replacing a service
program requires `*CHANGE`; binding or loading it will require `*USE` once the
runtime linker exists. Until that linker exists, `CALL` continues to require a
`*PGM` with a valid `user.l400.toolchain.manifest`.
