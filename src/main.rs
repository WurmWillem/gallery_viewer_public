use std::io::{self, Write};

use dropbox_sdk::default_client::UserAuthDefaultClient;
use dropbox_sdk::files::{self};
use dropbox_sdk::oauth2::{Authorization, AuthorizeUrlBuilder, Oauth2Type, PkceCode};
use iced::alignment::{Horizontal, Vertical};
use iced::widget::image::Handle;
use iced::widget::{column, container, text, Image, Text};
use iced::{Alignment, Application, Command, Element, Length, Settings, Theme};

const TIME_TO_SWAP: u64 = 5;

fn main() -> iced::Result {
    <GalleryViewer as Application>::run(Settings::default())
}

struct GalleryViewer {
    theme: Theme,
    value: usize,
    images: Vec<Handle>,
    time_till_next_img: std::time::Instant,
}

#[derive(Debug, Clone)]
enum Message {
    SwapImg,
    CloudImagesLoaded(Vec<Handle>),
}

impl Application for GalleryViewer {
    type Message = Message;
    type Executor = iced::executor::Default;
    type Theme = Theme;
    type Flags = ();

    fn title(&self) -> String {
        String::from("Gallery Viewer")
    }

    fn update(&mut self, message: Message) -> iced::Command<Message> {
        // println!("{message:?}");
        match message {
            Message::SwapImg => {
                if self.time_till_next_img.elapsed().as_secs_f32() >= TIME_TO_SWAP as f32 {
                    self.time_till_next_img = std::time::Instant::now();
                    if self.images.len() > 1 {
                        if self.value == self.images.len() - 1 {
                            self.value = 0;
                        } else {
                            self.value += 1;
                        }
                    }
                }
            }
            Message::CloudImagesLoaded(handles) => {
                println!("Cloud images loaded in memory!");

                self.images = handles;
            }
        }

        Command::none()
    }

    fn view(&self) -> Element<Message> {
        let image_or_empty: Element<_> = match self.images.get(self.value) {
            Some(handle) => Image::new(handle.clone())
                .width(Length::Fill)
                .height(Length::Fill)
                .into(),
            None => Text::new("Downloading images!").into(),
        };

        let time_left =
            (TIME_TO_SWAP as f32 - self.time_till_next_img.elapsed().as_secs_f32()) as i32 + 1;

        let time_left = text(time_left)
            .size(20)
            .horizontal_alignment(Horizontal::Center)
            .vertical_alignment(Vertical::Center);

        let content = column![image_or_empty, time_left,]
            .spacing(20)
            .padding(20)
            .align_items(Alignment::Center);

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x()
            .center_y()
            .into()
    }

    fn subscription(&self) -> iced::Subscription<Self::Message> {
        iced::time::every(std::time::Duration::from_millis(100)).map(|_| Message::SwapImg)
    }

    fn theme(&self) -> Theme {
        self.theme.clone()
    }

    fn new(_flags: Self::Flags) -> (Self, iced::Command<Self::Message>) {
        let me = GalleryViewer {
            images: vec![],
            time_till_next_img: std::time::Instant::now(),
            theme: Theme::Dark,
            value: 0,
        };
        (
            me,
            Command::batch([
                Command::perform(load_data(), Message::CloudImagesLoaded),
                iced::window::change_mode(
                    iced::window::Id::unique(),
                    iced::window::Mode::Fullscreen,
                ),
            ]),
        )
    }
}

async fn load_data() -> Vec<Handle> {
    let client_id = "m35223alvo00gb2";
    let oauth2_flow = Oauth2Type::PKCE(PkceCode::new());
    let url = AuthorizeUrlBuilder::new(client_id, &oauth2_flow).build();
    eprintln!("Open this URL in your browser:");
    eprintln!("{}", url);
    eprintln!();
    let auth_code = prompt("Then paste the code here");

    let auth = Authorization::from_auth_code(
        client_id.to_string(),
        oauth2_flow,
        auth_code.trim().to_owned(),
        None,
    );
    println!("{auth:?}");
    let client = UserAuthDefaultClient::new(auth);

    let result = match files::list_folder(
        &client,
        &files::ListFolderArg::new(String::new()).with_recursive(true),
    ) {
        Ok(Ok(result)) => result,
        Ok(Err(e)) => {
            panic!("Error from files/list_folder: {e}");
        }
        Err(e) => {
            panic!("API request error: {e}");
        }
    };

    let endings = [".jpg", ".jpeg", ".png"];

    let images_path: Vec<String> = result
        .entries
        .iter()
        .filter_map(|a| {
            if let files::Metadata::File(file) = a {
                if endings.iter().any(|n| file.name.ends_with(*n)) {
                    return file.path_lower.clone();
                }
            }
            None
        })
        .collect();

    println!("{images_path:#?}");

    let mut images = vec![vec![]; images_path.len()];

    for (index, image_path) in images_path.into_iter().enumerate() {
        match files::download(&client, &files::DownloadArg::new(image_path), None, None) {
            Ok(Ok(result)) => {
                let _ = io::copy(
                    &mut result.body.expect("there must be a response body"),
                    &mut images[index],
                )
                .expect("I/O error");
            }
            Ok(Err(e)) => {
                eprintln!("Error from files/download: {e}");
            }
            Err(e) => {
                eprintln!("API request error: {e}");
            }
        }
    }

    images.into_iter().map(Handle::from_memory).collect()
}

fn prompt(msg: &str) -> String {
    eprint!("{}: ", msg);
    io::stderr().flush().unwrap();
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    input.trim().to_owned()
}
