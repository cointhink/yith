use lettre::{SmtpClient, Transport};
use lettre_email::Email;

pub fn send(to_addr: &str, subject: &str, body: &str) {
    let payload = format!("Subject: {}\n\n{}", subject, body);
    let email = Email::builder() // to be used
        .to(to_addr)
        .from("yith@donp.org")
        .subject(subject)
        .body(body)
        .build()
        .unwrap();
    let mut mailer = SmtpClient::new_unencrypted_localhost().unwrap().transport();
    let result = mailer.send(email.into());
    let word = if result.is_ok() { "sent" } else { "FAILED" };
    println!("email {} {} {}", to_addr, subject, word);
}
