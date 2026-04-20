/* addlible_demo.clp — Añade QGPL a la library list */
PGM
    ADDLIBLE LIB(QGPL)
    CHGCURLIB CURLIB(QGPL)
    SNDPGMMSG MSG('Library list actualizada.')
ENDPGM
