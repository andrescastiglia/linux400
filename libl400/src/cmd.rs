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
const OBJ_PARAMS: &[CommandParameter] = &[
    CommandParameter {
        name: "OBJ",
        type_: "NAME",
        required: false,
        values: "*ALL or LIB/OBJ",
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
const OBJ_REQUIRED_PARAMS: &[CommandParameter] = &[CommandParameter {
    name: "OBJ",
    type_: "NAME",
    required: true,
    values: "LIB/OBJ",
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
const CMD_PARAMS: &[CommandParameter] = &[CommandParameter {
    name: "CMD",
    type_: "NAME",
    required: true,
    values: "command name",
    default: "",
}];

pub const COMMAND_METADATA: &[CommandMetadata] = &[
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
        parameters: OBJ_REQUIRED_PARAMS,
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
        name: "DSPDTAQ",
        text: "Display data queue",
        authority: "*USE",
        parameters: DTAQ_PARAMS,
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
                name: "TEXT",
                type_: "CHAR",
                required: false,
                values: "description",
                default: "User command",
            },
        ],
    },
];

pub fn command_metadata(name: &str) -> Option<&'static CommandMetadata> {
    let name = name.trim().to_uppercase();
    COMMAND_METADATA
        .iter()
        .find(|metadata| metadata.name == name.as_str())
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
