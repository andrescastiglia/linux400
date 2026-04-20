use anyhow::Result;
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use l400::{
    assign_to_workload, create_l400_slices, register_current_job, remove_job, update_job_status,
    WorkloadType, cgroup::JobStatus,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

fn main() -> Result<()> {
    // La TUI es un workload interactivo, pero la falla en cgroups no debe impedir el login.
    let _ = create_l400_slices();
    let _ = assign_to_workload(std::process::id() as u64, WorkloadType::Interactive);
    let _ = register_current_job(
        "OS400-TUI",
        WorkloadType::Interactive,
        JobStatus::Active,
        "os400-tui",
    );
    let pid = std::process::id() as u64;

    let mut terminal = setup_terminal()?;
    let result = run_app(&mut terminal);
    restore_terminal()?;
    let _ = update_job_status(pid, JobStatus::Completed);
    let _ = remove_job(pid);
    result
}

fn setup_terminal() -> Result<Terminal<CrosstermBackend<std::io::Stdout>>> {
    use std::io::stdout;

    execute!(stdout(), EnterAlternateScreen)?;
    enable_raw_mode()?;

    let backend = CrosstermBackend::new(stdout());
    let terminal = Terminal::new(backend)?;
    Ok(terminal)
}

fn restore_terminal() -> Result<()> {
    use std::io::stdout;

    execute!(stdout(), LeaveAlternateScreen)?;
    disable_raw_mode()?;
    Ok(())
}

fn run_app<T: ratatui::backend::Backend>(terminal: &mut Terminal<T>) -> Result<()> {
    let mut app = os400_tui::App::new();
    app.run(terminal)?;
    Ok(())
}
