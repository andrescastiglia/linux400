#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommandParameter {
    pub name: &'static str,
    pub type_: &'static str,
    pub required: bool,
    pub values: &'static str,
    pub default: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommandMetadata {
    pub name: &'static str,
    pub text: &'static str,
    pub authority: &'static str,
    pub parameters: &'static [CommandParameter],
}

const NO_PARAMS: &[CommandParameter] = &[];
const JOB_FILTER_PARAMS: &[CommandParameter] = &[
    CommandParameter {
        name: "SBS",
        type_: "NAME",
        required: false,
        values: "subsystem name",
        default: "*ALL",
    },
    CommandParameter {
        name: "SUBSYSTEM",
        type_: "NAME",
        required: false,
        values: "subsystem name",
        default: "*ALL",
    },
    CommandParameter {
        name: "STATUS",
        type_: "CHAR",
        required: false,
        values: "*ALL,*ACTIVE,*JOBQ,*COMPLETED,*FAILED,*HELD",
        default: "*ALL",
    },
    CommandParameter {
        name: "OPTION",
        type_: "CHAR",
        required: false,
        values: "work option",
        default: "",
    },
    CommandParameter {
        name: "PID",
        type_: "DEC",
        required: false,
        values: "process id",
        default: "",
    },
    CommandParameter {
        name: "JOB",
        type_: "NAME",
        required: false,
        values: "job name",
        default: "",
    },
];
const JOB_ACTION_PARAMS: &[CommandParameter] = &[
    CommandParameter {
        name: "JOB",
        type_: "NAME",
        required: false,
        values: "job name",
        default: "",
    },
    CommandParameter {
        name: "PID",
        type_: "DEC",
        required: false,
        values: "process id",
        default: "",
    },
];
const ENDJOB_PARAMS: &[CommandParameter] = &[
    CommandParameter {
        name: "JOB",
        type_: "NAME",
        required: false,
        values: "job name",
        default: "",
    },
    CommandParameter {
        name: "PID",
        type_: "DEC",
        required: false,
        values: "process id",
        default: "",
    },
    CommandParameter {
        name: "CONFIRM",
        type_: "CHAR",
        required: false,
        values: "*YES,*NO",
        default: "*NO",
    },
];
const OBJ_PARAMS: &[CommandParameter] = &[
    CommandParameter {
        name: "OBJ",
        type_: "NAME",
        required: false,
        values: "*ALL or LIB/OBJ",
        default: "*ALL",
    },
    CommandParameter {
        name: "FILTER",
        type_: "NAME",
        required: false,
        values: "object filter",
        default: "*ALL",
    },
    CommandParameter {
        name: "OBJTYPE",
        type_: "CHAR",
        required: false,
        values: "*ALL,*PGM,*FILE,*DTAQ,*CMD,*LIB,*OUTQ",
        default: "*ALL",
    },
    CommandParameter {
        name: "LIB",
        type_: "NAME",
        required: false,
        values: "library name",
        default: "*ALL",
    },
];
const OBJ_REQUIRED_PARAMS: &[CommandParameter] = &[
    CommandParameter {
        name: "OBJ",
        type_: "NAME",
        required: true,
        values: "LIB/OBJ",
        default: "",
    },
    CommandParameter {
        name: "OBJTYPE",
        type_: "CHAR",
        required: false,
        values: "*PGM,*FILE,*DTAQ,*CMD,*LIB,*OUTQ,*ALL",
        default: "*ALL",
    },
    CommandParameter {
        name: "LIB",
        type_: "NAME",
        required: false,
        values: "library name",
        default: "*LIBL",
    },
];
const OBJ_DELETE_PARAMS: &[CommandParameter] = &[
    CommandParameter {
        name: "OBJ",
        type_: "NAME",
        required: true,
        values: "LIB/OBJ",
        default: "",
    },
    CommandParameter {
        name: "OBJTYPE",
        type_: "CHAR",
        required: false,
        values: "*PGM,*FILE,*DTAQ,*CMD,*LIB,*OUTQ,*ALL",
        default: "*ALL",
    },
    CommandParameter {
        name: "LIB",
        type_: "NAME",
        required: false,
        values: "library name",
        default: "*LIBL",
    },
    CommandParameter {
        name: "CONFIRM",
        type_: "CHAR",
        required: false,
        values: "*YES,*NO",
        default: "*NO",
    },
];
const COPY_OBJ_PARAMS: &[CommandParameter] = &[
    CommandParameter {
        name: "OBJ",
        type_: "NAME",
        required: true,
        values: "source object",
        default: "",
    },
    CommandParameter {
        name: "TOOBJ",
        type_: "NAME",
        required: true,
        values: "target object",
        default: "",
    },
    CommandParameter {
        name: "LIB",
        type_: "NAME",
        required: false,
        values: "source library",
        default: "*LIBL",
    },
    CommandParameter {
        name: "TOLIB",
        type_: "NAME",
        required: false,
        values: "target library",
        default: "*CURLIB",
    },
    CommandParameter {
        name: "OBJTYPE",
        type_: "CHAR",
        required: false,
        values: "*PGM,*FILE,*DTAQ,*CMD,*LIB,*OUTQ,*ALL",
        default: "*ALL",
    },
];
const CHANGE_OBJ_PARAMS: &[CommandParameter] = &[
    CommandParameter {
        name: "OBJ",
        type_: "NAME",
        required: true,
        values: "LIB/OBJ",
        default: "",
    },
    CommandParameter {
        name: "OBJTYPE",
        type_: "CHAR",
        required: false,
        values: "*PGM,*FILE,*DTAQ,*CMD,*LIB,*OUTQ,*ALL",
        default: "*ALL",
    },
    CommandParameter {
        name: "LIB",
        type_: "NAME",
        required: false,
        values: "library name",
        default: "*LIBL",
    },
    CommandParameter {
        name: "TEXT",
        type_: "CHAR",
        required: false,
        values: "object text",
        default: "",
    },
    CommandParameter {
        name: "OBJATTR",
        type_: "CHAR",
        required: false,
        values: "object attribute",
        default: "",
    },
];
const OBJ_AUTH_PARAMS: &[CommandParameter] = &[
    CommandParameter {
        name: "OBJ",
        type_: "NAME",
        required: true,
        values: "LIB/OBJ",
        default: "",
    },
    CommandParameter {
        name: "LIB",
        type_: "NAME",
        required: false,
        values: "library name",
        default: "*LIBL",
    },
    CommandParameter {
        name: "OBJTYPE",
        type_: "CHAR",
        required: false,
        values: "*PGM,*FILE,*DTAQ,*CMD,*LIB,*OUTQ,*ALL",
        default: "*ALL",
    },
];
const GRANT_AUTH_PARAMS: &[CommandParameter] = &[
    CommandParameter {
        name: "OBJ",
        type_: "NAME",
        required: true,
        values: "LIB/OBJ",
        default: "",
    },
    CommandParameter {
        name: "LIB",
        type_: "NAME",
        required: false,
        values: "library name",
        default: "*LIBL",
    },
    CommandParameter {
        name: "OBJTYPE",
        type_: "CHAR",
        required: false,
        values: "*PGM,*FILE,*DTAQ,*CMD,*LIB,*OUTQ,*ALL",
        default: "*ALL",
    },
    CommandParameter {
        name: "USER",
        type_: "NAME",
        required: true,
        values: "user profile",
        default: "",
    },
    CommandParameter {
        name: "AUT",
        type_: "CHAR",
        required: true,
        values: "*USE,*CHANGE,*ALL,*EXCLUDE",
        default: "*USE",
    },
];
const REVOKE_AUTH_PARAMS: &[CommandParameter] = &[
    CommandParameter {
        name: "OBJ",
        type_: "NAME",
        required: true,
        values: "LIB/OBJ",
        default: "",
    },
    CommandParameter {
        name: "LIB",
        type_: "NAME",
        required: false,
        values: "library name",
        default: "*LIBL",
    },
    CommandParameter {
        name: "OBJTYPE",
        type_: "CHAR",
        required: false,
        values: "*PGM,*FILE,*DTAQ,*CMD,*LIB,*OUTQ,*ALL",
        default: "*ALL",
    },
    CommandParameter {
        name: "USER",
        type_: "NAME",
        required: true,
        values: "user profile",
        default: "",
    },
];
const CHECK_AUTH_PARAMS: &[CommandParameter] = &[
    CommandParameter {
        name: "OBJ",
        type_: "NAME",
        required: true,
        values: "LIB/OBJ",
        default: "",
    },
    CommandParameter {
        name: "LIB",
        type_: "NAME",
        required: false,
        values: "library name",
        default: "*LIBL",
    },
    CommandParameter {
        name: "OBJTYPE",
        type_: "CHAR",
        required: false,
        values: "*PGM,*FILE,*DTAQ,*CMD,*LIB,*OUTQ,*ALL",
        default: "*ALL",
    },
    CommandParameter {
        name: "USER",
        type_: "NAME",
        required: false,
        values: "user profile",
        default: "*CURRENT",
    },
    CommandParameter {
        name: "AUT",
        type_: "CHAR",
        required: false,
        values: "*USE,*CHANGE,*ALL,*EXCLUDE",
        default: "*USE",
    },
];
const LIB_PARAMS: &[CommandParameter] = &[CommandParameter {
    name: "LIB",
    type_: "NAME",
    required: true,
    values: "library name",
    default: "",
}];
const RENAME_OBJ_PARAMS: &[CommandParameter] = &[
    CommandParameter {
        name: "OBJ",
        type_: "NAME",
        required: true,
        values: "current object name",
        default: "",
    },
    CommandParameter {
        name: "NEWNAME",
        type_: "NAME",
        required: true,
        values: "new object name",
        default: "",
    },
];
const PGM_PARAMS: &[CommandParameter] = &[CommandParameter {
    name: "PGM",
    type_: "NAME",
    required: true,
    values: "LIB/PGM",
    default: "",
}];
const CLPGM_PARAMS: &[CommandParameter] = &[
    CommandParameter {
        name: "PGM",
        type_: "NAME",
        required: true,
        values: "LIB/PGM",
        default: "",
    },
    CommandParameter {
        name: "SRCFILE",
        type_: "NAME",
        required: false,
        values: "LIB/FILE",
        default: "QGPL/QCLSRC",
    },
    CommandParameter {
        name: "SRCMBR",
        type_: "NAME",
        required: false,
        values: "member name",
        default: "MAIN.CLP",
    },
];
const MENU_PARAMS: &[CommandParameter] = &[CommandParameter {
    name: "MENU",
    type_: "NAME",
    required: true,
    values: "menu name",
    default: "MAIN",
}];
const SEU_PARAMS: &[CommandParameter] = &[
    CommandParameter {
        name: "FILE",
        type_: "NAME",
        required: true,
        values: "LIB/FILE",
        default: "QGPL/QCLSRC",
    },
    CommandParameter {
        name: "MBR",
        type_: "NAME",
        required: true,
        values: "member name",
        default: "",
    },
];
const SQL_PARAMS: &[CommandParameter] = &[CommandParameter {
    name: "STMT",
    type_: "CHAR",
    required: false,
    values: "SQL statement",
    default: "",
}];
const FILE_PARAMS: &[CommandParameter] = &[
    CommandParameter {
        name: "FILE",
        type_: "NAME",
        required: true,
        values: "LIB/FILE",
        default: "",
    },
    CommandParameter {
        name: "MBR",
        type_: "NAME",
        required: false,
        values: "member name",
        default: "PF_MEMBER",
    },
];
const FILE_CREATE_PARAMS: &[CommandParameter] = &[
    CommandParameter {
        name: "FILE",
        type_: "NAME",
        required: true,
        values: "LIB/FILE",
        default: "",
    },
    CommandParameter {
        name: "LIB",
        type_: "NAME",
        required: false,
        values: "library name",
        default: "*CURLIB",
    },
    CommandParameter {
        name: "RCDLEN",
        type_: "DEC",
        required: false,
        values: "record length",
        default: "80",
    },
    CommandParameter {
        name: "FIELDS",
        type_: "CHAR",
        required: false,
        values: "field definition list",
        default: "",
    },
    CommandParameter {
        name: "KEY",
        type_: "CHAR",
        required: false,
        values: "key field",
        default: "",
    },
    CommandParameter {
        name: "TEXT",
        type_: "CHAR",
        required: false,
        values: "description",
        default: "",
    },
];
const LF_CREATE_PARAMS: &[CommandParameter] = &[
    CommandParameter {
        name: "FILE",
        type_: "NAME",
        required: true,
        values: "LIB/FILE",
        default: "",
    },
    CommandParameter {
        name: "LIB",
        type_: "NAME",
        required: false,
        values: "library name",
        default: "*CURLIB",
    },
    CommandParameter {
        name: "SRCFILE",
        type_: "NAME",
        required: false,
        values: "LIB/FILE",
        default: "",
    },
    CommandParameter {
        name: "SRCLIB",
        type_: "NAME",
        required: false,
        values: "source library",
        default: "*LIBL",
    },
    CommandParameter {
        name: "KEY",
        type_: "CHAR",
        required: false,
        values: "key expression",
        default: "",
    },
    CommandParameter {
        name: "TEXT",
        type_: "CHAR",
        required: false,
        values: "description",
        default: "",
    },
];
const FILE_CONFIRM_PARAMS: &[CommandParameter] = &[
    CommandParameter {
        name: "FILE",
        type_: "NAME",
        required: true,
        values: "LIB/FILE",
        default: "",
    },
    CommandParameter {
        name: "LIB",
        type_: "NAME",
        required: false,
        values: "library name",
        default: "*LIBL",
    },
    CommandParameter {
        name: "MBR",
        type_: "NAME",
        required: false,
        values: "member name",
        default: "PF_MEMBER",
    },
    CommandParameter {
        name: "CONFIRM",
        type_: "CHAR",
        required: false,
        values: "*YES,*NO",
        default: "*NO",
    },
];
const WRITE_FILE_PARAMS: &[CommandParameter] = &[
    CommandParameter {
        name: "FILE",
        type_: "NAME",
        required: true,
        values: "LIB/FILE",
        default: "",
    },
    CommandParameter {
        name: "LIB",
        type_: "NAME",
        required: false,
        values: "library name",
        default: "*LIBL",
    },
    CommandParameter {
        name: "MBR",
        type_: "NAME",
        required: false,
        values: "member name",
        default: "PF_MEMBER",
    },
    CommandParameter {
        name: "KEY",
        type_: "CHAR",
        required: false,
        values: "record key",
        default: "",
    },
    CommandParameter {
        name: "DATA",
        type_: "CHAR",
        required: true,
        values: "record data",
        default: "",
    },
];
const MEMBER_DELETE_PARAMS: &[CommandParameter] = &[
    CommandParameter {
        name: "FILE",
        type_: "NAME",
        required: true,
        values: "LIB/FILE",
        default: "",
    },
    CommandParameter {
        name: "MBR",
        type_: "NAME",
        required: true,
        values: "member name",
        default: "",
    },
    CommandParameter {
        name: "CONFIRM",
        type_: "CHAR",
        required: false,
        values: "*YES,*NO",
        default: "*NO",
    },
];
const MEMBER_COPY_PARAMS: &[CommandParameter] = &[
    CommandParameter {
        name: "FILE",
        type_: "NAME",
        required: true,
        values: "LIB/FILE",
        default: "",
    },
    CommandParameter {
        name: "MBR",
        type_: "NAME",
        required: true,
        values: "source member",
        default: "",
    },
    CommandParameter {
        name: "TOMBR",
        type_: "NAME",
        required: true,
        values: "target member",
        default: "",
    },
];
const MEMBER_CHANGE_PARAMS: &[CommandParameter] = &[
    CommandParameter {
        name: "FILE",
        type_: "NAME",
        required: true,
        values: "LIB/FILE",
        default: "",
    },
    CommandParameter {
        name: "MBR",
        type_: "NAME",
        required: true,
        values: "member name",
        default: "",
    },
    CommandParameter {
        name: "TEXT",
        type_: "CHAR",
        required: false,
        values: "member text",
        default: "",
    },
];
const DTAQ_PARAMS: &[CommandParameter] = &[
    CommandParameter {
        name: "DTAQ",
        type_: "NAME",
        required: true,
        values: "LIB/DTAQ",
        default: "",
    },
    CommandParameter {
        name: "MSG",
        type_: "CHAR",
        required: false,
        values: "message text",
        default: "",
    },
];
const DTAQ_CREATE_PARAMS: &[CommandParameter] = &[
    CommandParameter {
        name: "DTAQ",
        type_: "NAME",
        required: true,
        values: "LIB/DTAQ",
        default: "",
    },
    CommandParameter {
        name: "LIB",
        type_: "NAME",
        required: false,
        values: "library name",
        default: "*CURLIB",
    },
];
const DTAQ_RECEIVE_PARAMS: &[CommandParameter] = &[
    CommandParameter {
        name: "DTAQ",
        type_: "NAME",
        required: true,
        values: "LIB/DTAQ",
        default: "",
    },
    CommandParameter {
        name: "LIB",
        type_: "NAME",
        required: false,
        values: "library name",
        default: "*LIBL",
    },
    CommandParameter {
        name: "WAIT",
        type_: "DEC",
        required: false,
        values: "seconds",
        default: "0",
    },
];
const CMD_PARAMS: &[CommandParameter] = &[CommandParameter {
    name: "CMD",
    type_: "NAME",
    required: true,
    values: "command name",
    default: "",
}];
const USER_PROFILE_PARAMS: &[CommandParameter] = &[
    CommandParameter {
        name: "USRPRF",
        type_: "NAME",
        required: false,
        values: "user profile",
        default: "*ALL",
    },
    CommandParameter {
        name: "ACTION",
        type_: "CHAR",
        required: false,
        values: "*LIST,*DISPLAY",
        default: "*LIST",
    },
];
const OUTQ_CREATE_PARAMS: &[CommandParameter] = &[
    CommandParameter {
        name: "OUTQ",
        type_: "NAME",
        required: true,
        values: "LIB/OUTQ",
        default: "",
    },
    CommandParameter {
        name: "LIB",
        type_: "NAME",
        required: false,
        values: "library name",
        default: "*CURLIB",
    },
    CommandParameter {
        name: "TEXT",
        type_: "CHAR",
        required: false,
        values: "description",
        default: "",
    },
];
const OUTQ_DELETE_PARAMS: &[CommandParameter] = &[
    CommandParameter {
        name: "OUTQ",
        type_: "NAME",
        required: true,
        values: "LIB/OUTQ",
        default: "",
    },
    CommandParameter {
        name: "LIB",
        type_: "NAME",
        required: false,
        values: "library name",
        default: "*LIBL",
    },
    CommandParameter {
        name: "CONFIRM",
        type_: "CHAR",
        required: false,
        values: "*YES,*NO",
        default: "*NO",
    },
];
const SPLF_PARAMS: &[CommandParameter] = &[
    CommandParameter {
        name: "SPLF",
        type_: "NAME",
        required: true,
        values: "spool file id",
        default: "",
    },
    CommandParameter {
        name: "FILE",
        type_: "NAME",
        required: false,
        values: "spool file name",
        default: "",
    },
];
const SPLF_DELETE_PARAMS: &[CommandParameter] = &[
    CommandParameter {
        name: "SPLF",
        type_: "NAME",
        required: true,
        values: "spool file id",
        default: "",
    },
    CommandParameter {
        name: "FILE",
        type_: "NAME",
        required: false,
        values: "spool file name",
        default: "",
    },
    CommandParameter {
        name: "CONFIRM",
        type_: "CHAR",
        required: false,
        values: "*YES,*NO",
        default: "*NO",
    },
];
const POWER_PARAMS: &[CommandParameter] = &[
    CommandParameter {
        name: "OPTION",
        type_: "CHAR",
        required: false,
        values: "*CNTRLD,*IMMED,*RESTART",
        default: "*CNTRLD",
    },
    CommandParameter {
        name: "CONFIRM",
        type_: "CHAR",
        required: true,
        values: "*YES",
        default: "*NO",
    },
];
const SUBMIT_JOB_PARAMS: &[CommandParameter] = &[
    CommandParameter {
        name: "CMD",
        type_: "CHAR",
        required: true,
        values: "command string",
        default: "",
    },
    CommandParameter {
        name: "JOB",
        type_: "NAME",
        required: false,
        values: "job name",
        default: "QBATCH",
    },
    CommandParameter {
        name: "JOBQ",
        type_: "NAME",
        required: false,
        values: "job queue",
        default: "QBATCH",
    },
];

pub const COMMAND_METADATA: &[CommandMetadata] = &[
    CommandMetadata {
        name: "WRKSYSSTS",
        text: "Work with system status",
        authority: "*USE",
        parameters: NO_PARAMS,
    },
    CommandMetadata {
        name: "WRKACTJOB",
        text: "Work with active jobs",
        authority: "*USE",
        parameters: JOB_FILTER_PARAMS,
    },
    CommandMetadata {
        name: "WRKJOBQ",
        text: "Work with job queues",
        authority: "*USE",
        parameters: NO_PARAMS,
    },
    CommandMetadata {
        name: "HLDJOB",
        text: "Hold job",
        authority: "*CHANGE",
        parameters: JOB_ACTION_PARAMS,
    },
    CommandMetadata {
        name: "RLSJOB",
        text: "Release job",
        authority: "*CHANGE",
        parameters: JOB_ACTION_PARAMS,
    },
    CommandMetadata {
        name: "ENDJOB",
        text: "End job",
        authority: "*CHANGE",
        parameters: ENDJOB_PARAMS,
    },
    CommandMetadata {
        name: "WRKSYSVAL",
        text: "Work with system values",
        authority: "*USE",
        parameters: NO_PARAMS,
    },
    CommandMetadata {
        name: "DSPLOG",
        text: "Display log",
        authority: "*USE",
        parameters: NO_PARAMS,
    },
    CommandMetadata {
        name: "DSPCMD",
        text: "Display command metadata",
        authority: "*USE",
        parameters: CMD_PARAMS,
    },
    CommandMetadata {
        name: "WRKCMD",
        text: "Work with command objects",
        authority: "*USE",
        parameters: NO_PARAMS,
    },
    CommandMetadata {
        name: "CRTCMD",
        text: "Create command object",
        authority: "*ALL",
        parameters: &[
            CommandParameter {
                name: "CMD",
                type_: "NAME",
                required: true,
                values: "LIB/CMD or CMD",
                default: "",
            },
            CommandParameter {
                name: "LIB",
                type_: "NAME",
                required: false,
                values: "library name",
                default: "QSYS",
            },
            CommandParameter {
                name: "TEXT",
                type_: "CHAR",
                required: false,
                values: "description",
                default: "User command",
            },
        ],
    },
    CommandMetadata {
        name: "WRKUSRPRF",
        text: "Work with user profiles",
        authority: "*USE",
        parameters: USER_PROFILE_PARAMS,
    },
    CommandMetadata {
        name: "WRKSPLF",
        text: "Work with spool files",
        authority: "*USE",
        parameters: NO_PARAMS,
    },
    CommandMetadata {
        name: "WRKOUTQ",
        text: "Work with output queues",
        authority: "*USE",
        parameters: NO_PARAMS,
    },
    CommandMetadata {
        name: "CRTOUTQ",
        text: "Create output queue",
        authority: "*CHANGE",
        parameters: OUTQ_CREATE_PARAMS,
    },
    CommandMetadata {
        name: "DLTOUTQ",
        text: "Delete output queue",
        authority: "*ALL",
        parameters: OUTQ_DELETE_PARAMS,
    },
    CommandMetadata {
        name: "DSPSPLF",
        text: "Display spool file",
        authority: "*USE",
        parameters: SPLF_PARAMS,
    },
    CommandMetadata {
        name: "DLTSPLF",
        text: "Delete spool file",
        authority: "*CHANGE",
        parameters: SPLF_DELETE_PARAMS,
    },
    CommandMetadata {
        name: "PWRDWNSYS",
        text: "Power down system",
        authority: "*ALL",
        parameters: POWER_PARAMS,
    },
    CommandMetadata {
        name: "SBMJOB",
        text: "Submit job",
        authority: "*CHANGE",
        parameters: SUBMIT_JOB_PARAMS,
    },
    CommandMetadata {
        name: "WRKOBJ",
        text: "Work with objects",
        authority: "*USE",
        parameters: OBJ_PARAMS,
    },
    CommandMetadata {
        name: "DSPOBJD",
        text: "Display object description",
        authority: "*USE",
        parameters: OBJ_REQUIRED_PARAMS,
    },
    CommandMetadata {
        name: "DSPOBJAUT",
        text: "Display object authority",
        authority: "*USE",
        parameters: OBJ_AUTH_PARAMS,
    },
    CommandMetadata {
        name: "GRTOBJAUT",
        text: "Grant object authority",
        authority: "*ALL",
        parameters: GRANT_AUTH_PARAMS,
    },
    CommandMetadata {
        name: "RVKOBJAUT",
        text: "Revoke object authority",
        authority: "*ALL",
        parameters: REVOKE_AUTH_PARAMS,
    },
    CommandMetadata {
        name: "CHKOBJAUT",
        text: "Check object authority",
        authority: "*USE",
        parameters: CHECK_AUTH_PARAMS,
    },
    CommandMetadata {
        name: "DSPPOLICY",
        text: "Display security policy",
        authority: "*USE",
        parameters: NO_PARAMS,
    },
    CommandMetadata {
        name: "DSPAUD",
        text: "Display audit log",
        authority: "*USE",
        parameters: NO_PARAMS,
    },
    CommandMetadata {
        name: "CRTLIB",
        text: "Create library",
        authority: "*CHANGE",
        parameters: LIB_PARAMS,
    },
    CommandMetadata {
        name: "DLTLIB",
        text: "Delete library",
        authority: "*ALL",
        parameters: LIB_PARAMS,
    },
    CommandMetadata {
        name: "ADDLIBLE",
        text: "Add library list entry",
        authority: "*USE",
        parameters: LIB_PARAMS,
    },
    CommandMetadata {
        name: "CHGCURLIB",
        text: "Change current library",
        authority: "*USE",
        parameters: LIB_PARAMS,
    },
    CommandMetadata {
        name: "RNMOBJ",
        text: "Rename object",
        authority: "*ALL",
        parameters: RENAME_OBJ_PARAMS,
    },
    CommandMetadata {
        name: "DLTOBJ",
        text: "Delete object",
        authority: "*ALL",
        parameters: OBJ_DELETE_PARAMS,
    },
    CommandMetadata {
        name: "CPYOBJ",
        text: "Copy object",
        authority: "*CHANGE",
        parameters: COPY_OBJ_PARAMS,
    },
    CommandMetadata {
        name: "CHGOBJD",
        text: "Change object description",
        authority: "*ALL",
        parameters: CHANGE_OBJ_PARAMS,
    },
    CommandMetadata {
        name: "CRTPGM",
        text: "Create program",
        authority: "*CHANGE",
        parameters: PGM_PARAMS,
    },
    CommandMetadata {
        name: "CRTCLPGM",
        text: "Create CL program",
        authority: "*CHANGE",
        parameters: CLPGM_PARAMS,
    },
    CommandMetadata {
        name: "CALL",
        text: "Call program",
        authority: "*USE",
        parameters: PGM_PARAMS,
    },
    CommandMetadata {
        name: "GO",
        text: "Show menu",
        authority: "*USE",
        parameters: MENU_PARAMS,
    },
    CommandMetadata {
        name: "SIGNOFF",
        text: "Sign off",
        authority: "*USE",
        parameters: NO_PARAMS,
    },
    CommandMetadata {
        name: "STRPDM",
        text: "Start PDM",
        authority: "*USE",
        parameters: NO_PARAMS,
    },
    CommandMetadata {
        name: "STRSEU",
        text: "Start SEU",
        authority: "*USE",
        parameters: SEU_PARAMS,
    },
    CommandMetadata {
        name: "STRSQL",
        text: "Start SQL",
        authority: "*USE",
        parameters: SQL_PARAMS,
    },
    CommandMetadata {
        name: "DSPPFM",
        text: "Display physical file member",
        authority: "*USE",
        parameters: FILE_PARAMS,
    },
    CommandMetadata {
        name: "WRKMBRPDM",
        text: "Work with members using PDM",
        authority: "*USE",
        parameters: FILE_PARAMS,
    },
    CommandMetadata {
        name: "DLTMBR",
        text: "Delete physical file member",
        authority: "*ALL",
        parameters: MEMBER_DELETE_PARAMS,
    },
    CommandMetadata {
        name: "CPYMBR",
        text: "Copy physical file member",
        authority: "*CHANGE",
        parameters: MEMBER_COPY_PARAMS,
    },
    CommandMetadata {
        name: "CHGMBRD",
        text: "Change member description",
        authority: "*CHANGE",
        parameters: MEMBER_CHANGE_PARAMS,
    },
    CommandMetadata {
        name: "CRTPF",
        text: "Create physical file",
        authority: "*CHANGE",
        parameters: FILE_CREATE_PARAMS,
    },
    CommandMetadata {
        name: "CRTLF",
        text: "Create logical file",
        authority: "*CHANGE",
        parameters: LF_CREATE_PARAMS,
    },
    CommandMetadata {
        name: "CLRPFM",
        text: "Clear physical file member",
        authority: "*ALL",
        parameters: FILE_CONFIRM_PARAMS,
    },
    CommandMetadata {
        name: "ADDPFM",
        text: "Add physical file member",
        authority: "*CHANGE",
        parameters: FILE_PARAMS,
    },
    CommandMetadata {
        name: "WRTPFM",
        text: "Write physical file member record",
        authority: "*CHANGE",
        parameters: WRITE_FILE_PARAMS,
    },
    CommandMetadata {
        name: "CRTDTAQ",
        text: "Create data queue",
        authority: "*CHANGE",
        parameters: DTAQ_CREATE_PARAMS,
    },
    CommandMetadata {
        name: "SNDDTAQ",
        text: "Send data queue message",
        authority: "*CHANGE",
        parameters: DTAQ_PARAMS,
    },
    CommandMetadata {
        name: "RCVDTAQ",
        text: "Receive data queue message",
        authority: "*CHANGE",
        parameters: DTAQ_RECEIVE_PARAMS,
    },
    CommandMetadata {
        name: "DSPDTAQ",
        text: "Display data queue",
        authority: "*USE",
        parameters: DTAQ_PARAMS,
    },
];

pub fn command_metadata(name: &str) -> Option<&'static CommandMetadata> {
    let name = name.trim().to_uppercase();
    COMMAND_METADATA
        .iter()
        .find(|metadata| metadata.name == name.as_str())
}

#[cfg(test)]
mod tests {
    use super::{command_metadata, COMMAND_METADATA};
    use std::collections::HashSet;

    const DISPATCHED_COMMANDS: &[&str] = &[
        "WRKSYSSTS",
        "WRKACTJOB",
        "WRKJOBQ",
        "HLDJOB",
        "RLSJOB",
        "ENDJOB",
        "WRKSYSVAL",
        "DSPLOG",
        "DSPCMD",
        "WRKCMD",
        "CRTCMD",
        "WRKUSRPRF",
        "WRKSPLF",
        "WRKOUTQ",
        "CRTOUTQ",
        "DLTOUTQ",
        "DSPSPLF",
        "DLTSPLF",
        "PWRDWNSYS",
        "SBMJOB",
        "WRKOBJ",
        "DLTOBJ",
        "CPYOBJ",
        "DSPOBJD",
        "CHGOBJD",
        "DSPOBJAUT",
        "GRTOBJAUT",
        "RVKOBJAUT",
        "CHKOBJAUT",
        "DSPPOLICY",
        "DSPAUD",
        "CRTLIB",
        "DLTLIB",
        "ADDLIBLE",
        "CHGCURLIB",
        "RNMOBJ",
        "CRTPGM",
        "CRTCLPGM",
        "CALL",
        "GO",
        "SIGNOFF",
        "STRPDM",
        "STRSEU",
        "STRSQL",
        "WRKMBRPDM",
        "DLTMBR",
        "CPYMBR",
        "CHGMBRD",
        "CRTPF",
        "CRTLF",
        "DSPPFM",
        "CLRPFM",
        "ADDPFM",
        "WRTPFM",
        "CRTDTAQ",
        "SNDDTAQ",
        "RCVDTAQ",
        "DSPDTAQ",
    ];

    #[test]
    fn metadata_covers_all_dispatched_commands() {
        for command in DISPATCHED_COMMANDS {
            assert!(
                command_metadata(command).is_some(),
                "missing command metadata for {command}"
            );
        }
    }

    #[test]
    fn command_metadata_names_are_unique() {
        let mut names = HashSet::new();
        for metadata in COMMAND_METADATA {
            assert!(
                names.insert(metadata.name),
                "duplicate command metadata for {}",
                metadata.name
            );
        }
    }
}

pub fn format_command_params(metadata: &CommandMetadata) -> String {
    metadata
        .parameters
        .iter()
        .map(|param| {
            format!(
                "{}:{}:{}:{}:{}",
                param.name,
                param.type_,
                if param.required {
                    "required"
                } else {
                    "optional"
                },
                param.values,
                param.default
            )
        })
        .collect::<Vec<_>>()
        .join("|")
}
