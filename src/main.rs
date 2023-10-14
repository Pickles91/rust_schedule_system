use std::collections::VecDeque;

use log::Log;
use process::{Burst, BurstKind, Process};
use scheduler::{Scheduler, fcfs::FCFS};
use system_state::SystemState;


mod process;
mod system_state;
mod scheduler;
mod log;

fn main() {
    let mut args = std::env::args();
    let _ = args.next().unwrap();
    let file = match args.next() {
        Some(v) => v,
        None => panic!("Please pass in a file name"),
    };
    let content = std::fs::read_to_string(&file).unwrap();
    let mut processes: Vec<Process> = content
        .split("\n")
        .enumerate()
        .map(|(pid, line)| {
            let mut process_info = line.split(" ");
            let name = process_info.next().unwrap();
            let arrival_time = process_info.next().unwrap().parse().unwrap();
            let priority = process_info.next().unwrap().parse().unwrap();
            let mut next = BurstKind::Cpu;
            let mut bursts = vec![];
            while let Some(v) = process_info.next() {
                bursts.push(Burst(next, v.parse().unwrap()));
                next = match next {
                    BurstKind::Cpu => BurstKind::Io,
                    BurstKind::Io => BurstKind::Cpu,
                };
            }
            dbg!(Process::new(name.to_owned(), pid.try_into().unwrap(), priority, bursts, arrival_time))
        })
        .collect();
    // sort them to be sorted by arrival time, since we only want to add them to the scheduler once they're in.
    processes.sort_unstable_by(|proc, other_proc| proc.arrival.cmp(&other_proc.arrival));

    // this is somewhat bad design, both CPU and IO schedulers share a type (willfully, it lets me reuse code)
    // but instead of storing the BurstKind as a field, it probably would of been better to make a type like
    // BurstKindCpu<FCFS> and BurstKindIo<FCFS>. Oh well. That would of had it's own complexities.
    // ...I can just do a runtime check to validate them but that's not hip and cool.
    start_sim(processes.into_iter().collect(), FCFS::new(vec![], BurstKind::Cpu), FCFS::new(vec![], BurstKind::Io));
}


fn start_sim(mut processes: VecDeque<Process>, mut cpu_sched: impl Scheduler, mut io_sched: impl Scheduler) {
    let mut finished_process_queue = vec![];

    let mut log = Log::new();
    
    let mut state = SystemState::new();

    loop {
        let mut arrived_processes = vec![];
        let mut cpu_process = None;
        let mut io_process = None;

        match processes.front() {
            Some(proc) if proc.arrival <= state.time => {
                arrived_processes.push(proc.clone());
                cpu_sched.enqueue(processes.pop_front().unwrap());
                continue;
            }
            _ => {},
        }

        let mut cpu_queue = vec![];
        let mut io_queue = vec![];

        // duplicated code (with subtle differences that makes abstracting this annoying)
        // alert.
        match cpu_sched.tick(&state) {
            scheduler::SchedulerResult::Finished(p) if p.burst.len() == 0 =>  {
                cpu_process = Some(p.clone());
                finished_process_queue.push(p.clone());
            },
            scheduler::SchedulerResult::Finished(p) => {
                cpu_process = Some(p.clone());
                match p.burst[0].0 {
                    BurstKind::Cpu => cpu_queue.push(p),
                    BurstKind::Io => io_queue.push(p),
                }
            }
            scheduler::SchedulerResult::Processing(p) => {
                cpu_process = Some(p.clone());
            },
            scheduler::SchedulerResult::Idle => {},
            scheduler::SchedulerResult::WrongKind => panic!("schedule for IO instead you idiot."),
            scheduler::SchedulerResult::NoBurstLeft => {},
        };
        match io_sched.tick(&state) {
            scheduler::SchedulerResult::Finished(p) if p.burst.len() == 0 =>  {
                io_process = Some(p.clone());
                finished_process_queue.push(p.clone());
            },
            scheduler::SchedulerResult::Finished(p) => {
                io_process = Some(p.clone());
                match p.burst[0].0 {
                    BurstKind::Cpu => cpu_queue.push(p),
                    BurstKind::Io => io_queue.push(p),
                }
            }
            scheduler::SchedulerResult::Processing(p) => {
                io_process = Some(p.clone());
            },
            scheduler::SchedulerResult::Idle => {},
            scheduler::SchedulerResult::WrongKind => panic!("schedule for IO instead you idiot."),
            scheduler::SchedulerResult::NoBurstLeft => {},
        };

        for i in cpu_queue { cpu_sched.enqueue(i); }
        for i in io_queue { io_sched.enqueue(i); }

        log.push(log::TickEntry { 
            cpu_process,
            io_process,
            arrived_processes,
            cpu_queue: cpu_sched.get_queue().into_iter().cloned().collect(),
            io_queue: io_sched.get_queue().into_iter().cloned().collect(),
            yet_to_arrive: processes.clone().into_iter().collect(),
            finished_processes: finished_process_queue.clone(),
        });
        log.draw_gui();

        state.time += 1;

        if cpu_sched.get_queue().is_empty() && io_sched.get_queue().is_empty() && processes.is_empty() {
            return;
        }
    }
}
