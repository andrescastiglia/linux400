use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use os400_tui::SessionContext;
use os400_tui::screens::admin_views::AdminCommandView;
use os400_tui::screens::cmd_line::CommandLine;
use os400_tui::screens::main_menu::MainMenu;
use os400_tui::screens::object_browser::ObjectBrowser;
use os400_tui::screens::pdm_browser::PdmBrowser;
use os400_tui::screens::sign_on::SignOnScreen;
use os400_tui::screens::str_seu::StrSeu;
use os400_tui::screens::str_sql::StrSql;
use os400_tui::screens::work_mgmt::WorkManagement;
use os400_tui::screens::wrk_mbr_pdm::WrkMbrPdm;
use os400_tui::screens::{Screen, ScreenId};
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use tempfile::TempDir;

struct EnvGuard {
    key: &'static str,
    previous: Option<std::ffi::OsString>,
}

impl EnvGuard {
    fn set(key: &'static str, value: &std::path::Path) -> Self {
        Self::set_os(key, value.as_os_str())
    }

    fn set_os(key: &'static str, value: &std::ffi::OsStr) -> Self {
        let previous = std::env::var_os(key);
        unsafe {
            std::env::set_var(key, value);
        }
        Self { key, previous }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        unsafe {
            if let Some(previous) = &self.previous {
                std::env::set_var(self.key, previous);
            } else {
                std::env::remove_var(self.key);
            }
        }
    }
}

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn type_text(screen: &mut dyn Screen, text: &str) {
    for ch in text.chars() {
        screen.handle_key(key(KeyCode::Char(ch)));
    }
}

fn render_at(screen: &mut dyn Screen, width: u16, height: u16) {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).expect("terminal");
    terminal
        .draw(|frame| screen.render(frame))
        .expect("render smoke");
}

fn bootstrap_root() -> (TempDir, EnvGuard, EnvGuard) {
    let root = tempfile::tempdir().expect("root");
    let root_guard = EnvGuard::set("L400_ROOT", root.path());
    l400::bootstrap_l400_root(root.path()).expect("bootstrap");

    let spool = root.path().join("QUSRSYS").join("QSPL");
    std::fs::create_dir_all(&spool).expect("spool");
    std::fs::write(
        spool.join("QPRINT_000001"),
        "spool_version=1 status=READY\njob=PHASE6 user=QPGMR\nhello spool\n",
    )
    .expect("spool file");
    let spool_guard = EnvGuard::set("L400_SPOOL_DIR", &spool);
    (root, root_guard, spool_guard)
}

fn install_l400cmd_stub(root: &std::path::Path) -> EnvGuard {
    let bin = root.join("bin");
    std::fs::create_dir_all(&bin).expect("bin");
    let log = root.join("tui-command-log");
    let stub = bin.join("l400cmd");
    std::fs::write(
        &stub,
        format!(
            "#!/usr/bin/env sh\nprintf '%s\\n' \"$*\" >> '{}'\ncase \"$1\" in\n  CRTCLPGM) echo '[CRTCLPGM] QGPL/PHASE6 compiled.' ;;\n  CALL) echo '[CALL] QGPL/PHASE6 completed.' ;;\n  SBMJOB) echo '[SBMJOB] PHASE6 submitted.' ; printf '%s\\n' 'spool_version=1 status=READY job=PHASE6 user=QPGMR command=CALL' > '{}/QPRINT_PHASE6' ;;\n  *) echo '[l400cmd stub]' \"$*\" ;;\nesac\n",
            log.display(),
            root.join("QUSRSYS").join("QSPL").display()
        ),
    )
    .expect("stub");
    let mut permissions = std::fs::metadata(&stub).expect("metadata").permissions();
    use std::os::unix::fs::PermissionsExt;
    permissions.set_mode(0o755);
    std::fs::set_permissions(&stub, permissions).expect("chmod");

    let old_path = std::env::var_os("PATH").unwrap_or_default();
    let mut path = bin.into_os_string();
    path.push(":");
    path.push(old_path);
    EnvGuard::set_os("PATH", &path)
}

#[test]
fn phase6_screens_render_at_80_and_132_columns() {
    let (_root, _root_guard, _spool_guard) = bootstrap_root();
    let session = SessionContext::new(400);
    session.sign_on("QPGMR");

    let mut screens: Vec<Box<dyn Screen>> = vec![
        Box::new(SignOnScreen::new()),
        Box::new(MainMenu::with_session(session.clone())),
        Box::new(CommandLine::with_session(session.clone())),
        Box::new(ObjectBrowser::with_session(session.clone())),
        Box::new(PdmBrowser::with_session(session.clone())),
        Box::new(WrkMbrPdm::new("QGPL".to_string(), "QCLSRC".to_string())),
        Box::new(StrSeu::from_member_spec(
            "QGPL",
            "QCLSRC",
            "HELLO.CLP",
            ScreenId::WrkMbrPdm,
            Some("QGPL/QCLSRC".to_string()),
        )),
        Box::new(StrSql::with_session(
            Some("QGPL/QCLSRC".to_string()),
            ScreenId::WrkMbrPdm,
            Some("QGPL/QCLSRC".to_string()),
            session.clone(),
        )),
        Box::new(WorkManagement::new()),
        Box::new(AdminCommandView::spool_outq(None, session)),
    ];

    for screen in screens.iter_mut() {
        render_at(screen.as_mut(), 80, 24);
        render_at(screen.as_mut(), 132, 27);
    }
}

#[test]
fn phase6_automated_tui_flow_reaches_required_workflows() {
    let (_root, _root_guard, _spool_guard) = bootstrap_root();
    let session = SessionContext::new(401);
    session.sign_on("QPGMR");

    let mut menu = MainMenu::with_session(session.clone());
    assert_eq!(
        menu.handle_key(key(KeyCode::Char('6'))).next,
        Some(ScreenId::CommandLine)
    );
    assert_eq!(
        menu.handle_key(key(KeyCode::Char('7'))).next,
        Some(ScreenId::PdmBrowser)
    );
    assert_eq!(
        menu.handle_key(key(KeyCode::Char('4'))).next,
        Some(ScreenId::WorkManagement)
    );
    assert_eq!(menu.handle_key(key(KeyCode::Char('1'))).next, None);
    assert_eq!(
        menu.handle_key(key(KeyCode::Char('1'))).next,
        Some(ScreenId::SpoolOutq)
    );

    let mut cmd = CommandLine::with_session(session.clone());
    type_text(&mut cmd, "WRKOBJ");
    assert_eq!(
        cmd.handle_key(key(KeyCode::Enter)).next,
        Some(ScreenId::ObjectBrowser)
    );

    let mut cmd = CommandLine::with_session(session.clone());
    type_text(&mut cmd, "STRPDM");
    assert_eq!(
        cmd.handle_key(key(KeyCode::Enter)).next,
        Some(ScreenId::PdmBrowser)
    );

    let mut cmd = CommandLine::with_session(session.clone());
    type_text(&mut cmd, "STRSEU FILE(QGPL/QCLSRC) MBR(HELLO.CLP)");
    assert_eq!(
        cmd.handle_key(key(KeyCode::Enter)).next,
        Some(ScreenId::StrSeu)
    );

    let mut cmd = CommandLine::with_session(session.clone());
    type_text(&mut cmd, "STRSQL");
    assert_eq!(
        cmd.handle_key(key(KeyCode::Enter)).next,
        Some(ScreenId::StrSql)
    );

    let mut cmd = CommandLine::with_session(session.clone());
    type_text(&mut cmd, "WRKACTJOB");
    assert_eq!(
        cmd.handle_key(key(KeyCode::Enter)).next,
        Some(ScreenId::WorkManagement)
    );

    let mut cmd = CommandLine::with_session(session.clone());
    type_text(&mut cmd, "WRKSPLF");
    assert_eq!(
        cmd.handle_key(key(KeyCode::Enter)).next,
        Some(ScreenId::SpoolOutq)
    );

    let mut prompt = CommandLine::with_session(session.clone());
    type_text(&mut prompt, "CALL");
    assert_eq!(prompt.handle_key(key(KeyCode::F(4))).next, None);
    render_at(&mut prompt, 80, 24);

    let mut members = WrkMbrPdm::new("QGPL".to_string(), "QCLSRC".to_string());
    assert_eq!(members.handle_key(key(KeyCode::F(6))).next, None);
    type_text(&mut members, "PHASE6");
    assert_eq!(members.handle_key(key(KeyCode::Enter)).next, None);
    assert_eq!(
        members.handle_key(key(KeyCode::Char('2'))).next,
        Some(ScreenId::StrSeu)
    );

    let mut seu = StrSeu::from_member_spec(
        "QGPL",
        "QCLSRC",
        "PHASE6.CLP",
        ScreenId::WrkMbrPdm,
        Some("QGPL/QCLSRC".to_string()),
    );
    type_text(&mut seu, "PGM");
    seu.handle_key(key(KeyCode::Enter));
    type_text(&mut seu, "ENDPGM");
    assert_eq!(
        seu.handle_key(key(KeyCode::F(3))).next,
        Some(ScreenId::WrkMbrPdm)
    );
    assert!(
        l400::resolve_l400_root()
            .join("QGPL")
            .join("QCLSRC")
            .join("PHASE6.CLP")
            .exists()
    );

    let mut spool = AdminCommandView::spool_outq(None, session);
    render_at(&mut spool, 80, 24);
    assert_eq!(spool.handle_key(key(KeyCode::Char('5'))).next, None);
    render_at(&mut spool, 132, 27);
}

#[test]
fn phase6_create_compile_call_submit_and_spool_flow_is_automated() {
    let (root, _root_guard, _spool_guard) = bootstrap_root();
    let _path_guard = install_l400cmd_stub(root.path());
    let session = SessionContext::new(402);
    session.sign_on("QPGMR");

    let mut members = WrkMbrPdm::new("QGPL".to_string(), "QCLSRC".to_string());
    members.handle_key(key(KeyCode::F(6)));
    type_text(&mut members, "PHASE6B");
    members.handle_key(key(KeyCode::Enter));

    let mut seu = StrSeu::from_member_spec(
        "QGPL",
        "QCLSRC",
        "PHASE6B.CLP",
        ScreenId::WrkMbrPdm,
        Some("QGPL/QCLSRC".to_string()),
    );
    type_text(&mut seu, "PGM");
    seu.handle_key(key(KeyCode::Enter));
    type_text(&mut seu, "ENDPGM");
    seu.handle_key(key(KeyCode::F(3)));

    for command in [
        "CRTCLPGM PGM(QGPL/PHASE6) SRCFILE(QGPL/QCLSRC) SRCMBR(PHASE6B.CLP)",
        "CALL PGM(QGPL/PHASE6)",
        "SBMJOB CMD(CALL PGM(QGPL/PHASE6)) JOB(PHASE6) JOBQ(QBATCH)",
    ] {
        let mut cmd = CommandLine::with_session(session.clone());
        type_text(&mut cmd, command);
        assert_eq!(cmd.handle_key(key(KeyCode::Enter)).next, None);
        render_at(&mut cmd, 80, 24);
    }

    let log = std::fs::read_to_string(root.path().join("tui-command-log")).expect("command log");
    assert!(log.contains("CRTCLPGM PGM(QGPL/PHASE6)"));
    assert!(log.contains("CALL PGM(QGPL/PHASE6)"));
    assert!(log.contains("SBMJOB CMD(CALL PGM(QGPL/PHASE6))"));
    assert!(
        root.path()
            .join("QUSRSYS")
            .join("QSPL")
            .join("QPRINT_PHASE6")
            .exists()
    );

    let mut spool = AdminCommandView::spool_outq(None, session);
    render_at(&mut spool, 80, 24);
    assert_eq!(spool.handle_key(key(KeyCode::Char('5'))).next, None);
}
