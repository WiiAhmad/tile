[info] [health.checked] health is Healthy {}
[info] [telegram.command.received] health {"message_id":1212,"text_snippet":"/health"}
[info] [telegram.message.received] text {"message_id":1214,"text_snippet":"hello"}
[info] [telegram.message.sent] text {"reply_to_message_id":1214,"text_snippet":"Hello! 👋 How can I help?"}
[info] [telegram.message.received] text {"message_id":1216,"text_snippet":"what is ur name"}
[info] [telegram.message.sent] text {"reply_to_message_id":1216,"text_snippet":"I’m your Telegram AI assistant. You can call me Assistant 🙂"}
[info] [telegram.command.received] start {"message_id":1218,"text_snippet":"/start"}
[info] [health.checked] health is Healthy {}
[info] [telegram.command.received] help {"message_id":1220,"text_snippet":"/help"}
[info] [telegram.command.received] l0list {"message_id":1222,"text_snippet":"/l0list"}
[info] [telegram.command.received] l0search {"message_id":1224,"text_snippet":"/l0search"}
[info] [telegram.message.received] text {"message_id":1226,"text_snippet":"what is ur name"}
[info] [telegram.message.sent] text {"reply_to_message_id":1226,"text_snippet":"I’m your Telegram AI assistant. You can call me Assistant 🙂"}
[info] [health.checked] health is Healthy {}
[info] [telegram.message.received] text {"message_id":1228,"text_snippet":"how many i send u message to give you question about name"}
[info] [telegram.message.sent] text {"reply_to_message_id":1228,"text_snippet":"You asked me about my name 2 times."}
^C 2026-05-31T18:20:39.257Z INFO  teloxide::dispatching::dispatcher > ^C received, trying to shutdown the dispatcher...
 2026-05-31T18:20:39.257Z INFO  teloxide::utils::shutdown_token   > Trying to shutdown the dispatcher...
^C^C^C^C 2026-05-31T18:20:42.435Z INFO  teloxide::utils::shutdown_token   > Dispatching has been shut down.
 2026-05-31T18:20:42.435Z INFO  teloxide::dispatching::dispatcher > dispatcher is shutdown...
wii@localhost:~/code/bot$ cargo run
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.15s
     Running `target/debug/bot`
 2026-05-31T18:20:50.360Z DEBUG tungstenite::handshake::client > Client handshake done.
 2026-05-31T18:20:50.360Z DEBUG tungstenite::handshake::client > Client handshake done.
 2026-05-31T18:20:50.361Z DEBUG tungstenite::handshake::client > Client handshake done.
[info] [health.checked] health is Healthy {}
[info] [runtime.initialized] Telegram AI L0 bot initialized {}
Telegram AI L0 bot initialized
[info] [health.checked] health is Healthy {}
 2026-05-31T18:20:50.420Z DEBUG reqwest::connect               > starting new connection: https://api.telegram.org/
 2026-05-31T18:20:51.137Z DEBUG teloxide::dispatching::dispatcher > hinting allowed updates: [Message]
[info] [telegram.message.received] text {"message_id":1230,"text_snippet":"Hello"}
 2026-05-31T18:20:57.488Z DEBUG reqwest::connect                  > starting new connection: http://127.0.0.1:8317/
 2026-05-31T18:20:59.557Z DEBUG aisdk::core::client               > Request succeeded on attempt 1
 2026-05-31T18:20:59.559Z DEBUG reqwest::connect                  > starting new connection: https://api.telegram.org/
[info] [telegram.message.sent] text {"reply_to_message_id":1230,"text_snippet":"Hello! 👋 How can I help?"}
[info] [telegram.message.received] text {"message_id":1232,"text_snippet":"how many message i send u words \"hello\""}
 2026-05-31T18:21:27.921Z DEBUG reqwest::connect                  > starting new connection: http://127.0.0.1:8317/
 2026-05-31T18:21:31.985Z DEBUG aisdk::core::client               > Request succeeded on attempt 1
[info] [telegram.message.sent] text {"reply_to_message_id":1232,"text_snippet":"You sent messages containing “hello” 4 times."}