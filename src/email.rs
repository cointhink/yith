use lettre::{EmailAddress, Envelope, SendableEmail, SmtpClient, Transport};

pub fn send(to_addr: &str, subject: &str, body: &str) {
    let payload = format!("Subject: {}\n\n{}", subject, body);
    let email = SendableEmail::new(
        Envelope::new(
            Some(EmailAddress::new("yith@donp.org".to_string()).unwrap()),
            vec![EmailAddress::new(to_addr.to_string()).unwrap()],
        )
        .unwrap(),
        format!("msgid-acb123"),
        payload.into_bytes(),
    );
    let mut mailer = SmtpClient::new_unencrypted_localhost().unwrap().transport();
    let result = mailer.send(email);
    let word = if result.is_ok() { "sent" } else { "FAILED" };
    println!("email {} {} {}", to_addr, subject, word);
    println!("{}", body);
}
