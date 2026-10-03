fn print_init_script(shell: &str) {
    match shell {
        "bash" => {
            println!("{}", r#"
# cbox bash integration
_cbox_widget() {
  local out ret
  out="$(command cbox </dev/tty)"
  ret=$?
  if [ $ret -eq 1 ] || [ -z "$out" ]; then
    return
  fi

  if [ $ret -eq 2 ]; then
    READLINE_LINE="$out"
    READLINE_POINT=${#READLINE_LINE}
    bind '"\e[0n": accept-line'
    printf '\e[5n'
  else
    READLINE_LINE="$out"
    READLINE_POINT=${#READLINE_LINE}
  fi
}
bind -x '"\C-g": _cbox_widget' 2>/dev/null || true

cbox() {
  if [ $# -gt 0 ]; then
    command cbox "$@"
    return
  fi
  local out ret
  out="$(command cbox </dev/tty)"
  ret=$?
  if [ $ret -eq 1 ] || [ -z "$out" ]; then
    return
  fi
  if [ $ret -eq 2 ]; then
    history -s "$out"
    eval "$out"
  else
    history -s "$out"
    bind '"\e[0n": "'"$out"'"'
    printf '\e[5n'
  fi
}
"#);
        }
        "zsh" => {
            println!("{}", r#"
# cbox zsh integration
cbox_widget() {
  local out ret
  out="$(command cbox </dev/tty)"
  ret=$?
  if [ $ret -eq 1 ] || [ -z "$out" ]; then
    zle && zle redisplay
    return
  fi

  BUFFER="$out"
  CURSOR=${#BUFFER}
  if [ $ret -eq 2 ]; then
    zle accept-line
  fi
  zle && zle redisplay
}
zle -N cbox_widget
bindkey '^g' cbox_widget

cbox() {
  if [ $# -gt 0 ]; then
    command cbox "$@"
    return
  fi
  local out ret
  out="$(command cbox </dev/tty)"
  ret=$?
  if [ $ret -eq 1 ] || [ -z "$out" ]; then
    return
  fi
  if [ $ret -eq 2 ]; then
    print -s "$out"
    eval "$out"
  else
    print -z -- "$out"
  fi
}
"#);
        }
        "fish" => {
            println!("{}", r#"
# cbox fish integration
function _cbox_widget
    set -l out (command cbox </dev/tty)
    set -l ret $status
    if test $ret -eq 1; or test -z "$out"
        commandline -f repaint
        return
    end

    commandline -r -- $out
    commandline -C (string length -- $out)
    if test $ret -eq 2
        commandline -f execute
    end
    commandline -f repaint
end
bind \cg _cbox_widget

function cbox
    if count $argv > /dev/null
        command cbox $argv
        return
    end
    set -l out (command cbox </dev/tty)
    set -l ret $status
    if test $ret -eq 1; or test -z "$out"
        return
    end
    if test $ret -eq 2
        eval $out
    else
        echo $out
    end
end
"#);
        }
        "nushell" | "nu" => {
            println!("{}", r#"
# cbox nushell integration
# $env.config = ($env.config | upsert keybindings (
#     $env.config.keybindings | append {
#         name: cbox_widget
#         modifier: control
#         keycode: char_g
#         mode: [emacs, vi_normal, vi_insert]
#         event: {
#             send: executehostcommand
#             cmd: "
#                 let out = (^cbox < /dev/tty | complete);
#                 if $out.exit_code == 1 { return };
#                 let cmd_content = $out.stdout | str trim;
#                 if ($cmd_content | is-empty) { return };
#                 commandline edit --replace $cmd_content;
#             "
#         }
#     }
# ))
"#);
        }
        _ => {
            eprintln!("cbox: unsupported shell '{}'", shell);
        }
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() >= 3 && args[1] == "init" {
        print_init_script(&args[2]);
        return;
    }

    match run_tui() {
        Ok(TuiResult::Pick { entry, execute }) => {
            let (cmd_part, _) = split_command_comment(&entry.cmd);
            println!("{}", cmd_part);
            if execute {
                std::process::exit(2);
            } else {
                std::process::exit(0);
            }
        }
        Ok(TuiResult::Cancel) => {
            std::process::exit(1);
        }
        Err(e) => {
            eprintln!("cbox: {}", e);
            std::process::exit(1);
        }
    }
}
