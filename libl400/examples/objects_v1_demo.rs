use l400::{
    create_library, create_lf, create_object_with_metadata, create_pf, crtdtaq, list_objects,
    resolve_l400_root,
};
use std::env;
use std::path::PathBuf;

fn demo_root() -> PathBuf {
    env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(resolve_l400_root)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = demo_root();
    std::fs::create_dir_all(&root)?;

    let qsys = match create_library(&root, "QSYS") {
        Ok(path) => path,
        Err(l400::ObjectError::AlreadyExists) => root.join("QSYS"),
        Err(err) => return Err(Box::new(err)),
    };
    let qusrsys = match create_library(&root, "QUSRSYS") {
        Ok(path) => path,
        Err(l400::ObjectError::AlreadyExists) => root.join("QUSRSYS"),
        Err(err) => return Err(Box::new(err)),
    };

    let _usrprf = create_object_with_metadata(
        &qsys,
        "DEMOUSR",
        "*USRPRF",
        Some("USRPRF"),
        Some("Demo user profile"),
    )
    .or_else(|err| match err {
        l400::ObjectError::AlreadyExists => Ok(qsys.join("DEMOUSR")),
        other => Err(other),
    })?;

    let _pgm = create_object_with_metadata(
        &qsys,
        "HELLO",
        "*PGM",
        Some("C"),
        Some("Demo cataloged program"),
    )
    .or_else(|err| match err {
        l400::ObjectError::AlreadyExists => Ok(qsys.join("HELLO")),
        other => Err(other),
    })?;

    let pf = match create_pf(&qsys, "CUSTOMERS", 128) {
        Ok(pf) => pf,
        Err(l400::DbError::AlreadyExists) => l400::PhysicalFile::open(&qsys.join("CUSTOMERS"))?,
        Err(err) => return Err(Box::new(err)),
    };
    pf.write_rcd(b"C001", b"Ana,CABA")?;
    pf.write_rcd(b"C002", b"Luis,Rosario")?;

    let lf = match create_lf(&qsys, "CUSTBYNAME", &pf) {
        Ok(lf) => lf,
        Err(l400::DbError::AlreadyExists) => l400::LogicalFile::open(&qsys.join("CUSTBYNAME"))?,
        Err(err) => return Err(Box::new(err)),
    };
    let _ = lf.insert_idx(b"Ana", b"C001");
    let _ = lf.insert_idx(b"Luis", b"C002");

    let dtaq = match crtdtaq(&qusrsys, "QEZJOBLOG") {
        Ok(queue) => queue,
        Err(l400::DtaqError::AlreadyExists) => l400::DataQueue::open(&qusrsys.join("QEZJOBLOG"))?,
        Err(err) => return Err(Box::new(err)),
    };
    dtaq.snddtaq(b"Linux/400 demo message")?;

    println!("== Linux/400 Objects V1 Demo ==");
    println!("Root: {}", root.display());
    for library in [&qsys, &qusrsys] {
        let objects = list_objects(library)?;
        println!("Library {}:", library.file_name().unwrap_or_default().to_string_lossy());
        for object in objects {
            println!(
                "  - {} {} {} {}",
                object.name,
                object.objtype,
                object.attribute.unwrap_or_else(|| "-".to_string()),
                object.text.unwrap_or_default()
            );
        }
    }

    println!("PF CUSTOMERS records: {}", pf.read_all()?.len());
    println!("LF CUSTBYNAME index entries: {}", lf.read_all_idx()?.len());
    println!("DTAQ QEZJOBLOG messages: {}", dtaq.read_all()?.len());
    Ok(())
}
