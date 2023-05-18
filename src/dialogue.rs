use crate::commands::Command;
use crate::dialogue::format::print_diff;
use libtrainer_rs::record::Record;
use libtrainer_rs::task::Tasks;
use std::path::Path;
use teloxide::net::Download;
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup};
use teloxide::utils::command::BotCommands;
use teloxide::{dispatching::dialogue::InMemStorage, prelude::*};
use tokio::fs;
use tokio::fs::File;

type MyDialogue = Dialogue<State, InMemStorage<State>>;
type HandlerResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

mod format;

#[derive(Clone, Default, Debug)]
pub enum State {
    #[default]
    Start,
    ReceiveFile,
    RunTest {
        tasks: Tasks,
        answer: Option<Record>,
    },
    ReceiveField {
        tasks: Tasks,
        answer: Option<Record>,
    },
    ReceiveAns {
        tasks: Tasks,
        answer: Option<Record>,
        field: String,
    },
}

pub async fn start(bot: Bot, dialogue: MyDialogue, msg: Message) -> HandlerResult {
    bot.send_message(
        msg.chat.id,
        "Давай начнем! Пришли .csv файл с заданиями, чтобы начать.",
    )
    .await?;

    dialogue.update(State::ReceiveFile).await?;
    Ok(())
}

pub async fn receive_file(bot: Bot, dialogue: MyDialogue, msg: Message) -> HandlerResult {
    if let Some(doc) = msg.document() {
        let fpath = bot.get_file(doc.file.id.clone()).await?;

        let mut fout =
            File::create(Path::new(&format!("tmp/{}.csv", dialogue.chat_id().0))).await?;
        bot.download_file(&*fpath.path, &mut fout).await?;
        fout.sync_all().await?;

        let file = File::open(Path::new(&format!("tmp/{}.csv", dialogue.chat_id().0))).await?;

        let tasks = Tasks::from_csv(&file.into_std().await).unwrap_or_default();

        if tasks.len() == 0 {
            bot.send_message(dialogue.chat_id(), "В файле нет заданий")
                .await?;
        } else {
            dialogue
                .update(State::RunTest {
                    tasks,
                    answer: None,
                })
                .await?;
            bot.send_message(
                dialogue.chat_id(),
                "Файл загружен! Напишите что-нибудь, чтобы начать тренировку.",
            )
            .await?;
        }
        fs::remove_file(&format!("tmp/{}.csv", dialogue.chat_id().0)).await?;
    } else {
        bot.send_message(dialogue.chat_id(), "Пожалуйста, отправьте файл.")
            .await?;
    }

    Ok(())
}
pub async fn exit(bot: Bot, dialogue: MyDialogue, _msg: Message) -> HandlerResult {
    bot.send_message(dialogue.chat_id(), "Тест остановлен")
        .await?;
    dialogue.update(State::Start).await?;
    Ok(())
}

pub async fn run_test(
    bot: Bot,
    dialogue: MyDialogue,
    (mut tasks, mut answer): (Tasks, Option<Record>),
    msg: Message,
) -> HandlerResult {
    if answer.is_none() {
        let cur_task = tasks.get_random_task();
        answer = Some(Record::copy_format(cur_task.clone()));
    }

    if let Some(task) = answer.clone() {
        bot.send_message(msg.chat.id, task.to_string()).await?;

        let strings = task.clone().get_fields();
        let products = strings
            .iter()
            .map(|product| InlineKeyboardButton::callback(product, product));

        bot.send_message(msg.chat.id, "Выбери категорию:")
            .reply_markup(InlineKeyboardMarkup::new([products]).append_row([
                InlineKeyboardButton::callback("Сдать", "done"),
                InlineKeyboardButton::callback("Пропустить", "skip"),
            ]))
            .await?;
        dialogue
            .update(State::ReceiveField { tasks, answer })
            .await?;
    }

    Ok(())
}

pub async fn help(bot: Bot, msg: Message) -> HandlerResult {
    bot.send_message(msg.chat.id, Command::descriptions().to_string())
        .await?;
    Ok(())
}

pub async fn receive_type(
    bot: Bot,
    dialogue: MyDialogue,
    (tasks, answer): (Tasks, Option<Record>),
    q: CallbackQuery,
) -> HandlerResult {
    if let Some(field) = &q.data {
        if field == &"done".to_string() {
            let diff = tasks.check_answer(&answer.unwrap()).unwrap();

            let mut msg = print_diff(diff);
            if let Some(comment) = tasks.cur_task().unwrap().clone().comment() {
                msg += format!("Комментарий: {}", comment).as_str()
            }

            bot.send_message(dialogue.chat_id(), msg).await?;
            dialogue
                .update(State::RunTest {
                    tasks,
                    answer: None,
                })
                .await?;
        } else if field == &"skip".to_string() {
            dialogue
                .update(State::RunTest {
                    tasks,
                    answer: None,
                })
                .await?;
        } else {
            bot.send_message(dialogue.chat_id(), format!("Введи {field}:"))
                .await?;
            dialogue
                .update(State::ReceiveAns {
                    tasks,
                    answer,
                    field: field.clone(),
                })
                .await?;
        }
    }

    bot.answer_callback_query(q.id).await?;
    Ok(())
}

pub async fn receive_ans(
    bot: Bot,
    dialogue: MyDialogue,
    (tasks, answer, field): (Tasks, Option<Record>, String),
    msg: Message,
) -> HandlerResult {
    if let Some(ans) = msg.text() {
        if let Some(mut answer2) = answer {
            let mut splitted: Vec<String> = Vec::new();

            for i in ans.split(",") {
                splitted.push(i.to_string());
            }

            while tasks.cur_task().unwrap().clone().field_len(&field) > splitted.len() {
                splitted.push("".to_string());
            }

            splitted.sort();

            answer2.replace(&field, splitted);
            dialogue
                .update(State::ReceiveField {
                    tasks,
                    answer: Some(answer2),
                })
                .await?;
        }
    } else {
        bot.send_message(msg.chat.id, "Введите текст").await?;
    }

    Ok(())
}
