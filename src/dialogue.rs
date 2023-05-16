use crate::commands::Command;
use crate::dialogue::format::print_diff;
use libtrainer_rs::record::{diff, Record};
use libtrainer_rs::task::Tasks;
use std::path::Path;
use teloxide::net::Download;
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup};
use teloxide::utils::command::BotCommands;
use teloxide::{dispatching::dialogue::InMemStorage, prelude::*};
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
        cur_task: Option<Record>,
        answer: Option<Record>,
    },
    ReceiveField {
        tasks: Tasks,
        cur_task: Option<Record>,
        answer: Option<Record>,
    },
    ReceiveAns {
        tasks: Tasks,
        cur_task: Option<Record>,
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

        let mut fout = File::create(Path::new(&format!("{}.csv", dialogue.chat_id().0))).await?;
        bot.download_file(&*fpath.path, &mut fout).await?;
        fout.sync_all();

        let file = File::open(Path::new(&format!("{}.csv", dialogue.chat_id().0))).await?;

        let mut tasks = Tasks::from_csv(&file.into_std().await).unwrap_or_default();

        dialogue
            .update(State::RunTest {
                tasks,
                cur_task: None,
                answer: None,
            })
            .await?;
    } else {
        bot.send_message(dialogue.chat_id(), "Пожалуйста отправьте файл")
            .await?;
    }

    Ok(())
}
pub async fn exit(bot: Bot, dialogue: MyDialogue, msg: Message) -> HandlerResult {
    dbg!("exited");
    dialogue.exit().await?;
    Ok(())
}

pub async fn run_test(
    bot: Bot,
    dialogue: MyDialogue,
    (mut tasks, mut cur_task, mut answer): (Tasks, Option<Record>, Option<Record>),
    msg: Message,
) -> HandlerResult {
    if cur_task.is_none() {
        cur_task = Some(tasks.get_random_task().clone());
        answer = Some(Record::copy_format(cur_task.clone().unwrap()));
    }

    if let Some(task) = cur_task {
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
            .update(State::ReceiveField {
                tasks,
                cur_task: Some(task),
                answer,
            })
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
    (tasks, cur_task, answer): (Tasks, Option<Record>, Option<Record>),
    q: CallbackQuery,
) -> HandlerResult {
    if let Some(field) = &q.data {
        if field == &"done".to_string() {
            let diff = diff(&cur_task.clone().unwrap(), &answer.unwrap()).unwrap();

            let mut msg = print_diff(diff);
            if let Some(comment) = cur_task.unwrap().comment() {
                msg += format!("Комментарий: {}", comment).as_str()
            }

            bot.send_message(dialogue.chat_id(), msg).await?;
            dialogue
                .update(State::RunTest {
                    tasks,
                    cur_task: None,
                    answer: None,
                })
                .await?;
        } else if field == &"skip".to_string() {
            dialogue
                .update(State::RunTest {
                    tasks,
                    cur_task: None,
                    answer: None,
                })
                .await?;
        } else {
            bot.send_message(dialogue.chat_id(), format!("Введи {field}:"))
                .await?;
            dialogue
                .update(State::ReceiveAns {
                    tasks,
                    cur_task,
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
    (tasks, cur_task, answer, field): (Tasks, Option<Record>, Option<Record>, String),
    msg: Message,
) -> HandlerResult {
    if let Some(ans) = msg.text() {
        if let Some(mut answer2) = answer {
            let mut splitted: Vec<String> = Vec::new();

            for i in ans.split(",") {
                splitted.push(i.to_string());
            }

            while cur_task.clone().unwrap().field_len(&field) > splitted.len() {
                splitted.push("".to_string());
            }

            splitted.sort();

            answer2.replace(&field, splitted);
            dialogue
                .update(State::ReceiveField {
                    tasks,
                    cur_task,
                    answer: Some(answer2),
                })
                .await?;
        }
    } else {
        bot.send_message(msg.chat.id, "Введите текст").await?;
    }

    Ok(())
}
