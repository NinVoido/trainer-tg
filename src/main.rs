mod commands;
mod dialogue;

use crate::commands::Command;
use crate::dialogue::*;
use teloxide::dispatching::dialogue::enter;
use teloxide::dispatching::UpdateHandler;
use teloxide::{dispatching::dialogue::InMemStorage, prelude::*};

#[tokio::main]
async fn main() {
    let bot = Bot::from_env();

    Dispatcher::builder(bot, schema())
        .dependencies(dptree::deps![InMemStorage::<State>::new()])
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
}

fn schema() -> UpdateHandler<Box<dyn std::error::Error + Send + Sync + 'static>> {
    use dptree::case;

    let command_handler = teloxide::filter_command::<Command, _>()
        .branch(
            case![State::Start]
                .branch(case![Command::Help].endpoint(help))
                .branch(case![Command::Start].endpoint(start)),
        )
        .branch(case![Command::Exit].endpoint(exit));

    let message_handler = Update::filter_message()
        .branch(command_handler)
        .branch(case![State::ReceiveFile].endpoint(receive_file))
        .branch(case![State::RunTest { tasks, answer }].endpoint(run_test))
        .branch(
            case![State::ReceiveAns {
                tasks,
                answer,
                field
            }]
            .endpoint(receive_ans),
        );

    let callback_query_handler = Update::filter_callback_query()
        .branch(case![State::ReceiveField { tasks, answer }].endpoint(receive_type));

    enter::<Update, InMemStorage<State>, State, _>()
        .branch(message_handler)
        .branch(callback_query_handler)
}
