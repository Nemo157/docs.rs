use crate::Context;

use std::time::{Instant, Duration};

pub fn check_readmes(ctx: &dyn Context) -> anyhow::Result<()> {
    let mut conn = ctx.pool()?.get()?;

    let total: i64 = conn.query_one("select count(*) from releases inner join crates on releases.crate_id = crates.id where readme is not null and crates.name <> 'weakjson'", &[])?.get("count");

    let query = "
        select crates.name, releases.version, releases.readme
        from releases
        inner join crates on releases.crate_id = crates.id
        where releases.readme is not null and crates.name <> 'weakjson'
    ";

    let mut count: u32 = 0;
    let mut duration = Duration::ZERO;
    for row in conn.query(query, &[])? {
        let name: String = row.get("name");
        let version: String = row.get("version");
        let readme: String = row.get("readme");

        // println!("rendering {name}@{version}... ");
        let start = Instant::now();
        crate::web::markdown::render(&readme);
        let elapsed = start.elapsed();

        duration += elapsed;
        count += 1;

        // println!("...rendered in {elapsed:?} ({count}/{total} in {:?} avg)", duration / count);
    }

    println!("rendered {count}/{total} in {:?} on average", duration / count);

    Ok(())
}
