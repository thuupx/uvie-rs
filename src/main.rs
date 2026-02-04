use std::io::{self, Read};
use uvie::UltraFastViEngine;

fn main() {
    let mut engine = UltraFastViEngine::new();
    let mut stdin = io::stdin().lock();

    let mut buf = [0u8; 1];

    println!("Gõ thử Telex (Ctrl+C để thoát):");

    loop {
        // Đọc từng byte (giả sử chỉ demo với ASCII, không xử lý tổ hợp phím đặc biệt)
        if let Ok(n) = stdin.read(&mut buf) {
            if n == 0 {
                continue;
            }
            let b = buf[0];

            // Enter: xuống dòng, reset engine
            if b == b'\n' {
                let out = engine.feed(' ');
                println!("\n{}", out);
                continue;
            }

            // Thoát nếu là Ctrl+C (tuỳ bạn xử lý)
            if b == 3 {
                break;
            }

            let c = b as char;
            let out = engine.feed(c);
            // In kết quả hiện tại ra màn hình (giả lập behaviour “gõ tới đâu thấy tới đó”)
            print!("\r{}", out);
            io::Write::flush(&mut io::stdout()).unwrap();
        }
    }
}
