use rust_todo_cli::usecase::TodoUsecase;
use std::time::Instant;
use tempfile::tempdir;

#[test]
#[ignore] // 長時間かかるため通常時は無視
fn test_performance_scaling() {
    let scales = [100, 1000, 5000]; // 10000はJSON同期がボトルネックになる可能性があるため一旦5000まで

    for &n in &scales {
        let dir = tempdir().unwrap();
        let root = dir.path().to_path_buf();
        TodoUsecase::init(root.clone()).unwrap();
        let usecase = TodoUsecase::new(root).unwrap();

        println!("\n--- Scale: {} tasks ---", n);

        // 1. Add Performance (Sequential)
        let start = Instant::now();
        for i in 0..n {
            usecase.add_task(format!("Task {}", i), None, None).unwrap();
        }
        let duration = start.elapsed();
        println!(
            "Add {} tasks: {:?} (avg: {:?} / task)",
            n,
            duration,
            duration / n as u32
        );

        // 2. List (Flat) Performance
        let start = Instant::now();
        let tasks = usecase.list_tasks().unwrap();
        let duration = start.elapsed();
        println!("List (Flat) {} tasks: {:?}", n, duration);

        // 3. Tree Construction Performance (in-memory)
        // ここでは表示を除いたロジックのみを計測するためにusecaseの内部ロジックを模倣
        let start = Instant::now();
        let mut _count = 0;
        for task in &tasks {
            if task.parent_global_id.is_none() {
                _count += 1;
            }
        }
        let duration = start.elapsed();
        println!("Tree traversal (Simple) {} tasks: {:?}", n, duration);

        // 4. Sync Performance
        let start = Instant::now();
        usecase.sync().unwrap();
        let duration = start.elapsed();
        println!("Sync (Full) {} tasks: {:?}", n, duration);
    }
}
