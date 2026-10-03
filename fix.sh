sed -i '405s/\^x/^enter/' src/main.rs
sed -i '/(KeyCode::Char('\''x'\''), KeyModifiers::CONTROL)/,/^            }/d' src/main.rs
