#![allow(unused)]
use std::collections::VecDeque;
use rodio::*;
use std::path::{PathBuf};
use std::fs;
use std::fs::{DirEntry, File };
use crossterm::event;

use ratatui::layout::{Constraint,Direction, Layout, Rect,Alignment};
use ratatui::style::{Color, Modifier, Style, Stylize};
use ratatui::widgets::{List,ListItem,Block, ListState};
use ratatui::Frame;

use rand::prelude::IndexedRandom; // rand 0.9
                                  //
pub const SKIPTIME:u64 = 10;

pub struct Clock {
    last:std::time::Instant,
    acc:std::time::Duration,
    paused:bool,
}
impl Clock{
    fn new () -> Clock{
        Clock {
            last:std::time::Instant::now(),
            acc:std::time::Duration::from_secs(0),
            paused:true
        }
    }
    fn pause(&mut self) {
        if !self.paused { 
            self.acc += std::time::Instant::now() - self.last;
            self.paused = true;
        }
    }
    fn unpause(&mut self) {
        if self.paused { 
            self.last = std::time::Instant::now();
            self.paused = false;
        }
    }

    fn set (&mut self , d:std::time::Duration) {
        self.acc = d;
    }

    fn sub (&mut self , d:std::time::Duration) {
        self.acc -= d;
    }
    fn add (&mut self , d:std::time::Duration) {
        self.acc += d;
    }
    fn update(&mut self,d :std::time::Duration) {
        if d <= self.acc {
            *self= Clock::new();
            self.unpause();
        }
    }
    fn read(&self)->std::time::Duration {
        if self.paused {
            self.acc
        } else {
            self.acc + self.last.elapsed()
        } 
    }

}

pub struct Csource {
    s:String,
    d:std::time::Duration,
    p:PathBuf
}

impl Csource {
    fn new (s :String , d:std::time::Duration,p:PathBuf )-> Csource {
        Csource {
            s,d,p
        } 
    }

}
fn main() -> std::io::Result<()> {
    ratatui::run(| terminal| {
        let mut clock = Clock::new();
        let handle = DeviceSinkBuilder::open_default_sink().unwrap();

        let mut is_repeating  = false;
        let mut args = std::env::args();

        let config_path = format!("{}{}",std::env::home_dir().unwrap().as_os_str().to_str().unwrap(), "\\.config\\music_player\\path".to_owned());

        let binding = std::fs::read_to_string(&config_path).expect("no config found");
        let mut path = binding.trim();
        let mut toggle = false; 
        let mut a  :String = "".to_string();
        if let Some(arg )  = args.nth(1) {
            a = arg;
            toggle = true;
        }
        if toggle { 
            path = &a;
        }
        let player = rodio::Player::connect_new(handle.mixer());

        let mut sources:VecDeque<Csource> = VecDeque::new();

        let mut depth = 0;

        let mut entries:Vec<DirEntry> =  fs::read_dir(path).unwrap_or_else(|e| panic!("couldn't read path {}: {}", path, e)).map(|x:Result<DirEntry,_>| x.unwrap()).filter(|x| !x.file_name().into_string().unwrap().contains(".ini")).collect();
        entries.sort_by_key(|b| std::cmp::Reverse(b.file_type().unwrap().is_dir()));

        let mut random_dir:PathBuf = PathBuf::from(path);
        let mut is_random = false;

        let mut lists:Vec<(ListState,Vec<DirEntry>)> = vec![(ListState::default(),entries)];
        lists[0].0.select(Some(0));

        loop {
            while sources.len() > player.len() { 
                if !is_repeating { 
            if is_random {
                        if lists.is_empty() || lists[depth].1.is_empty() {continue;}

                        let mut entries:Vec<DirEntry> =  fs::read_dir(&random_dir)
                            .unwrap_or_else(|e| panic!("couldn't read path {}: {}", path, e))
                            .map(|x:Result<DirEntry,_>| x
                            .unwrap())
                            .filter(|x| !x
                                .file_name()
                                .into_string()
                                .unwrap()
                                .contains(".ini") &&
                                x.file_type().
                                unwrap().
                                is_file()
                                )
                            .collect();
                        if entries.is_empty() {continue;}
                        let entry = entries.choose(&mut rand::rng()).unwrap();

                        let file = File::open(entry.path()).unwrap(); // TODO CLEANUP

                        let source = if let Ok(p) = Decoder::try_from(file) {  // TODO CLEANUP
                                p 
                        }else{
                            continue; 
                        }; 
                        sources.clear();
                        sources.push_back(Csource::new(entry.file_name().into_string().unwrap(),source.total_duration().unwrap(),entry.path())); 
                        player.append(source); 
                        clock = Clock::new();
                        clock.unpause();

            }
            else {
                sources.pop_front();

                clock = Clock::new();

                if player.len() > 0 && !player.is_paused() {
                    clock.unpause();
                }
            }


                }else { 
                 let file = File::open(&sources.front().unwrap().p).unwrap();

                 let source = if let Ok(p) = Decoder::try_from(file) { 
                         p
                 }else{
                     continue;
                 };
                 clock = Clock::new();
                 clock.unpause();
                 sources.truncate(1);
                 player.clear();
                 player.append(source);
                 player.play();

                }
            } 

            terminal.draw(|frame | render(frame,&mut lists,depth,&sources,&mut clock,is_repeating,player.is_paused(),is_random))?;



            if event::poll(std::time::Duration::from_millis(16))? && let crossterm::event::Event::Key(key) = event::read()? && key.kind == crossterm::event::KeyEventKind::Press {

                match   key.code  {

                    crossterm::event::KeyCode::Char('q') => {break Ok(());},
                    crossterm::event::KeyCode::Char('a') => {
                        if   player.get_pos() > std::time::Duration::from_secs(SKIPTIME)   { 
                            let _ = player.try_seek(player.get_pos() - std::time::Duration::from_secs(SKIPTIME));
                            clock.sub(std::time::Duration::from_secs(SKIPTIME));
                        }else {
                            let _ = player.try_seek(std::time::Duration::from_secs(0));
                            clock.set(std::time::Duration::from_secs(0));
                        }
                    },
                    crossterm::event::KeyCode::Char('d') => {
                        let _ = player.try_seek(player.get_pos() + std::time::Duration::from_secs(SKIPTIME));
                        clock.add(std::time::Duration::from_secs(SKIPTIME));
                    },
                    crossterm::event::KeyCode::Char('0') => {
                        if !sources.is_empty()  {
                        let _ = player.try_seek(std::time::Duration::from_secs(0));
                        clock.set(std::time::Duration::from_secs(0));
                        }
                    },
                    crossterm::event::KeyCode::Char('1') => {
                        if !sources.is_empty()  {
                        let time = sources.front().unwrap().d.mul_f64(0.1);
                        let _ = player.try_seek(time);
                        clock.set(time);
                        }
                    },
                    crossterm::event::KeyCode::Char('2') => {
                        if !sources.is_empty()  {
                        let time = sources.front().unwrap().d.mul_f64(0.2);
                        let _ = player.try_seek(time);
                        clock.set(time);
                        }
                    },
                    crossterm::event::KeyCode::Char('3') => {
                        if !sources.is_empty()  {
                        let time = sources.front().unwrap().d.mul_f64(0.3);
                        let _ = player.try_seek(time);
                        clock.set(time);
                        }
                    },
                    crossterm::event::KeyCode::Char('4') => {
                        if !sources.is_empty()  {
                        let time = sources.front().unwrap().d.mul_f64(0.4);
                        let _ = player.try_seek(time);
                        clock.set(time);
                        }
                    },
                    crossterm::event::KeyCode::Char('5') => {
                        if !sources.is_empty()  {
                        let time = sources.front().unwrap().d.mul_f64(0.5);
                        let _ = player.try_seek(time);
                        clock.set(time);
                        }
                    },
                    crossterm::event::KeyCode::Char('6') => {
                        if !sources.is_empty()  {
                        let time = sources.front().unwrap().d.mul_f64(0.6);
                        let _ = player.try_seek(time);
                        clock.set(time);
                        }
                    },
                    crossterm::event::KeyCode::Char('7') => {
                        if !sources.is_empty()  {
                        let time = sources.front().unwrap().d.mul_f64(0.7);
                        let _ = player.try_seek(time);
                        clock.set(time);
                        }
                    },
                    crossterm::event::KeyCode::Char('8') => {
                        if !sources.is_empty()  {
                        let time = sources.front().unwrap().d.mul_f64(0.8);
                        let _ = player.try_seek(time);
                        clock.set(time);
                        }
                    },
                    crossterm::event::KeyCode::Char('9') => {
                        if !sources.is_empty()  {
                        let time = sources.front().unwrap().d.mul_f64(0.9);
                        let _ = player.try_seek(time);
                        clock.set(time);
                        }
                    },
                    crossterm::event::KeyCode::Char('w') => {
                        player.set_volume((player.volume() + 0.1).clamp(0.0,5.0));
                    },
                    crossterm::event::KeyCode::Char('s') => {
                        player.set_volume((player.volume() - 0.1).clamp(0.0,5.0));
                    },
                    crossterm::event::KeyCode::Char('h') => {
                        if depth == 0 {continue;}
                        lists.remove(lists.len()-1);
                        depth -=1;
                    },
                    crossterm::event::KeyCode::Char('l') => {
                        if lists[depth].1.is_empty()  {continue;}
                        let idx = lists[depth].0.selected().expect("TODO");
                        if lists[depth].1[idx].file_type().unwrap().is_file() {
                            continue;
                        }
                        let new_path= lists[depth].1[idx].path();
                        let mut new_list_state = ListState::default();
                        new_list_state.select_next();
                        let mut new_entries:Vec<DirEntry> =  fs::read_dir(new_path).unwrap().map(|x:Result<DirEntry,_>| x.unwrap()).collect(); 
                        new_entries.sort_by_key(|b| std::cmp::Reverse(b.file_type().unwrap().is_dir()));
                        lists.push((new_list_state,new_entries));
                        depth +=1;
                    },
                    crossterm::event::KeyCode::Char('j') => {
                        lists[depth].0.select_next();
                    },
                    crossterm::event::KeyCode::Char('k') => {
                        lists[depth].0.select_previous();
                    },
                    crossterm::event::KeyCode::Char('o') => {
                        if lists.is_empty() || lists[depth].1.is_empty() {continue;}
                        let idx = lists[depth].0.selected().unwrap();
                        let item = &lists[depth].1[idx];

                        let is_file = item.file_type().unwrap().is_file();
                        if !is_file {continue;}

                        let file = File::open(item.path()).unwrap();

                        let source = if let Ok(p) = Decoder::try_from(file) { 
                                p
                        }else{
                            continue;
                        };
                        is_random = false;
                        sources.push_back(Csource::new(item.file_name().into_string().unwrap(),source.total_duration().unwrap(),item.path()));
                        player.append(source);

                    },
                    crossterm::event::KeyCode::Char('n') => {
                        if !is_random { 
                        player.skip_one();
                        sources.pop_front();
                        clock = Clock::new();
                        clock.unpause();
                        is_repeating = false;
                        }else { 
                            let mut entries:Vec<DirEntry> =  fs::read_dir(&random_dir)
                            .unwrap_or_else(|e| panic!("couldn't read path {}: {}", path, e))
                            .map(|x:Result<DirEntry,_>| x
                            .unwrap())
                            .filter(|x| !x
                                .file_name()
                                .into_string()
                                .unwrap()
                                .contains(".ini") &&
                                x.file_type().
                                unwrap().
                                is_file()
                                )
                            .collect();
                        if entries.is_empty() {continue;}
                        let entry = entries.choose(&mut rand::rng()).unwrap();

                        let file = File::open(entry.path()).unwrap(); // TODO CLEANUP

                        let source = if let Ok(p) = Decoder::try_from(file) {  // TODO CLEANUP
                                p 
                        }else{
                            continue; 
                        }; 
                        sources.push_back(Csource::new(entry.file_name().into_string().unwrap(),source.total_duration().unwrap(),entry.path())); 
                        player.clear();
                        player.append(source); 
                        player.play();
                        clock = Clock::new();
                        clock.unpause();


                        }
                    },
                    crossterm::event::KeyCode::Char('p') => {
                        if !is_random   {

                        if lists.is_empty() || lists[depth].1.is_empty() {continue;}

                        let idx = lists[depth].0.selected().unwrap();
                        let item = &lists[depth].1[idx];

                        let is_dir = item.file_type().unwrap().is_dir();
                        if !is_dir {continue;}

                        random_dir = item.path();

                        let mut entries:Vec<DirEntry> =  fs::read_dir(&random_dir)
                            .unwrap_or_else(|e| panic!("couldn't read path {}: {}", path, e))
                            .map(|x:Result<DirEntry,_>| x
                            .unwrap())
                            .filter(|x| !x
                                .file_name()
                                .into_string()
                                .unwrap()
                                .contains(".ini") &&
                                x.file_type().
                                unwrap().
                                is_file()
                                )
                            .collect();
                        if entries.is_empty() {continue;}
                        let entry = entries.choose(&mut rand::rng()).unwrap();

                        let file = File::open(entry.path()).unwrap(); // TODO CLEANUP

                        let source = if let Ok(p) = Decoder::try_from(file) {  // TODO CLEANUP
                                p 
                        }else{
                            continue; 
                        }; 
                        sources.clear();
                        sources.push_back(Csource::new(entry.file_name().into_string().unwrap(),source.total_duration().unwrap(),entry.path())); 
                        player.clear();
                        player.append(source); 
                        player.play();
                        clock = Clock::new();
                        clock.unpause();
                        is_repeating = false;
                        }
                        is_random = !is_random;
                        is_repeating = false;
                    },
                    crossterm::event::KeyCode::Char('r') => {
                        if !is_repeating{
                        if sources.is_empty() {continue;}

                        let file = File::open(&sources.front().unwrap().p).unwrap();

                        let source = if let Ok(p) = Decoder::try_from(file) { 
                                p
                        }else{
                            continue;
                        };
                        sources.truncate(1);
                        player.clear();
                        player.append(source);
                        player.play();
                        let _ = player.try_seek(clock.read());
                    }
                        is_repeating = !is_repeating;
                    },
                    crossterm::event::KeyCode::Char('c') => {
                        player.clear();
                        sources = VecDeque::new();
                        clock = Clock::new();
                        is_repeating = false;
                    },
                    crossterm::event::KeyCode::Char(' ') => {
                        if player.is_paused() {
                            player.play();
                            clock.unpause();
                        }else {
                            player.pause();
                            clock.pause();
                        }
                    },


                    _ => {},
                }


                if !player.is_paused() {
                    clock.unpause();
                }
            }
        }
    })


}
fn render(frame: &mut Frame, lists:&mut[(ListState,Vec<DirEntry>)],depth:usize,sources:&VecDeque<Csource>,clock:&mut Clock,repeating:bool,paused:bool,random:bool) {

    let outermost = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Fill(1),
            Constraint::Max(3)
        ]).split(frame.area());

        let block  = Block::bordered();

        frame.render_widget(&block, outermost[1]);

        if let Some(sf) = sources.front() { 
            clock.update(sf.d);
            let gauge = ratatui::widgets::LineGauge::default()
                .filled_symbol(ratatui::symbols::line::THICK_HORIZONTAL)
                .label("")   
                .filled_symbol("█")  
                .unfilled_symbol(" ") 
            .ratio((clock.read().as_secs_f64()/ sf.d.as_secs_f64()).clamp(0.,1.));
            frame.render_widget(gauge, block.inner(outermost[1]));
        }



        let outer = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(75),
                Constraint::Percentage(25)
            ]).split(outermost[0]);
            let block  = Block::bordered().title_alignment(Alignment::Center).title("Player".bold().italic());

            frame.render_widget(&block,outer[1]);
            let mut ls = ListState::default();
            ls.select_next();

            let color = if paused {Color::Red}else{if repeating  {Color::Yellow} else {if random {Color::Magenta}else { Color::Blue }}};
            let sources:Vec<&str> = sources.iter().map(|x| x.s.as_str()).collect();
            let list = List::new(sources)
                .style(color)
                .highlight_style(Modifier::REVERSED)
                .highlight_symbol("");
    frame.render_stateful_widget(list,block.inner(outer[1]),&mut ls);


    let constraints = vec!(Constraint::Fill(1);depth + 1);
    //constraints.push(Constraint::Fill(5)); //TODO minimal auf 55 % runter gehtt das?

    let layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(constraints)
        .split(outer[0]);



    lists.iter_mut().enumerate().for_each(|(idx,(list_state,entries))|{ 
        let block  = Block::bordered();

        frame.render_widget(&block,layout[idx]);
        render_list(frame,block.inner(layout[idx]), list_state,entries);
    });
}


fn render_list(frame:&mut Frame , area: Rect, list_state: &mut ListState,items:&[DirEntry]){

    let items:Vec<ListItem> = items.iter().map(|x|{
        let name= x.file_name().into_string().unwrap();
        let is_dir = x.file_type().unwrap().is_dir();

        if is_dir { 
            ListItem::new(name)
                .style(Style::default().fg(Color::Green))
        }else {
            ListItem::new(name)
                .style(Style::default().fg(Color::White))
        }

    }).collect();


    let list = List::new(items)
        .style(Color::Green)
        .highlight_style(Modifier::REVERSED)
        .highlight_symbol("");
    frame.render_stateful_widget(list,area,list_state);

}


