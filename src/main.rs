use eframe::egui;
use egui::*;
use egui_plot::{PlotImage,Plot,PlotPoint};
use pdf2image::{RenderOptionsBuilder, PDF};
use std::env;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct Config {
    replace: Vec<(String, String)>,
    resolution: u32
}

impl Default for Config {
    fn default() -> Self {
        Config {
            replace: Vec::new(),
            resolution: 300
        }
    }
}

struct Doc {
    current_page: u32,
    page: Option<egui::TextureHandle>,
    dimension: (u32, u32),
    pdf: PDF,
    nb_pages: u32,
    index: String,
    search_up_to_date: bool,
    resolution: u32,
    replace: Vec<(String, String)>
}

struct Guiao {
    docs: Vec<Doc>,
    cd: usize,
    show_search: bool,
    search: String,
    last_search: String,
}

impl eframe::App for Guiao {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            let mut zoom = 1.;
            ctx.input_mut(|input| {
                if !self.show_search {
                    if input.consume_key(Modifiers::NONE, Key::R) && self.docs[self.cd].current_page < self.docs[self.cd].nb_pages {
                        self.docs[self.cd].current_page += 1;
                        self.docs[self.cd].page = None;
                    }
                    if input.consume_key(Modifiers::NONE, Key::C) && self.docs[self.cd].current_page>1 {
                        self.docs[self.cd].current_page -= 1;
                        self.docs[self.cd].page = None;
                    }
                    if input.consume_key(Modifiers::NONE, Key::Enter) {
                        self.show_search = true;
                    }
                    if input.consume_key(Modifiers::NONE, Key::Tab) {
                        self.cd = (self.cd+1)%self.docs.len();
                    }
                    if input.consume_key(Modifiers::NONE, Key::Plus) {
                        zoom *= 1.2;
                    }
                    if input.consume_key(Modifiers::NONE, Key::Minus) {
                        zoom /= 1.2;
                    }
                }
            });
            if self.show_search {
                let s = ui.add(egui::TextEdit::singleline(&mut self.search).desired_width(ui.available_width()));
                if s.lost_focus() {
                    self.show_search = false;
                    if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        self.last_search = self.search.clone().to_lowercase();
                        for doc in &mut self.docs {
                            doc.search_up_to_date = false;
                        }
                    }
                }
                s.request_focus();
            }
            if !self.docs[self.cd].search_up_to_date {
                let mut last_search = self.last_search.clone();
                for (s1, s2) in &self.docs[self.cd].replace {
                    last_search = last_search.replace(s1, s2);
                }
                let i = self.docs[self.cd].index.lines().rev().find(|l| *l.split(' ').next().unwrap()<=*last_search.as_str());
                let pos = if let Some(i) = i {
                    i.split(' ').last().unwrap().parse().unwrap()
                } else {
                    1
                };
                if pos > 0 && pos <= self.docs[self.cd].nb_pages {
                    if pos != self.docs[self.cd].current_page {
                        self.docs[self.cd].page = None;
                        self.docs[self.cd].current_page = pos;
                    }
                } else {
                    println!("Error: wrong index to page {}.", pos);
                }
                self.docs[self.cd].search_up_to_date = true;
            }
            if self.docs[self.cd].page.is_none() {
                let mut ro = RenderOptionsBuilder::default().build().unwrap();
                ro.resolution = pdf2image::DPI::Uniform(self.docs[self.cd].resolution);
                ro.pdftocairo = true;
                let image = self.docs[self.cd].pdf.render(pdf2image::Pages::Single(self.docs[self.cd].current_page), ro).unwrap();
                let image = image[0].clone().to_rgba8();
                let dim = image.dimensions();
                //println!("{} {}", dim.0, dim.1);
                let image = image.as_raw();
                let page = ColorImage::from_rgba_unmultiplied([dim.0.try_into().unwrap(), dim.1.try_into().unwrap()], image);
                let to = TextureOptions {
                    magnification: TextureFilter::Linear,
                    minification: TextureFilter::Linear,
                    wrap_mode: TextureWrapMode::ClampToEdge,
                    mipmap_mode: None
                };
                self.docs[self.cd].page = Some(ui.ctx().load_texture("page", page, to));
                self.docs[self.cd].dimension = dim;
            }
            
            let plot = Plot::new(self.cd).data_aspect(1.0).show_x(false).show_y(false).show_axes([false, false]).show_grid([false, false]).set_margin_fraction(Vec2 { x:0., y: 0.});
            let page = PlotImage::new(self.docs[self.cd].page.as_ref().unwrap(), PlotPoint::new(0., 0.), vec2(self.docs[self.cd].dimension.0 as f32, self.docs[self.cd].dimension.1 as f32));
            plot.show(ui, |plot_ui| {
                plot_ui.image(page);
                if zoom!=1. {
                    plot_ui.zoom_bounds(Vec2 { x: zoom, y: zoom }, PlotPoint { x: 0.0, y: 0.0 });
                }
            });
        });
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len()<2 {
        println!("usage: ./lexicon file1.pdf [file2.pdf ...]");
        return;
    }
    let mut docs = Vec::new();
    for path in args.iter().skip(1) {
        let path: String = path.parse().unwrap();
        let index_path = format!("{}.index", &path);
        let config_path = format!("{}.json", &path);
        let index = std::fs::read_to_string(&index_path).unwrap();
        let pdf = PDF::from_file(&path).unwrap();
        let nb_pages = pdf.page_count();
        let config_str = std::fs::read_to_string(&config_path).unwrap_or_else(|_| {serde_json::to_string(&Config::default()).unwrap()});
        let config: Config = serde_json::from_str(&config_str).unwrap();
        let doc = Doc {
            current_page: 1,
            page: None,
            dimension: (0, 0),
            pdf,
            nb_pages,
            index,
            search_up_to_date: true,
            resolution: config.resolution,
            replace: config.replace
        };
        docs.push(doc);
    }
    let guiao = Guiao {
        docs,
        cd: 0,
        show_search: false,
        search: String::new(),
        last_search: String::new(),
    };
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "Lexicon",
        options,
        Box::new(|cc| {
            let style = Style {
                visuals: Visuals::dark(),
                ..Style::default()
            };
            cc.egui_ctx.set_style(style);
            Ok(Box::new(guiao))
        }),
    )
    .unwrap();
}
