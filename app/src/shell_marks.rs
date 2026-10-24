//! Command marks for zsh and bash (DESIGN §13 P5-149): scripts that tell the terminal, with
//! OSC 133, where each prompt starts (`A`), where typing starts (`B`), where the command's output
//! starts (`C`) and that the command has ended, with its status (`D;<status>`). Plain shells get
//! them through a private `.zshenv` or rcfile that reads the user's own first; the Agent shell
//! appends them to its own (P5-61). The marks take no room, so the prompt looks as it did. The ones in the
//! prompt and `PS0` carry an option of their own (`;paddock=1`, ignored by the terminal) so the
//! scripts can tell them from another integration's: when the prompt already carries OSC 133
//! marks of its own (iTerm2, WezTerm), these stay out of its way.
use crate::agent_shell::Setup;
use anyhow::Result;
use std::{io::Write, os::unix::fs::OpenOptionsExt, path::Path};

/// Run at the end of the private `.zshenv`, once the user's own has been read. The prompt hook
/// moves itself last before each prompt, after whatever sets the prompt in `.zshrc`.
pub const ZSH: &str = r#"if [[ -o interactive && -z "${_paddock_marks-}" ]]; then
    typeset -g _paddock_marks=1 _paddock_marks_running=0 _paddock_marks_other=0 _paddock_marks_left=
    typeset -ga precmd_functions preexec_functions
    _paddock_marks_precmd() {
        local ret=$?
        if (( _paddock_marks_running )); then
            _paddock_marks_running=0
            (( _paddock_marks_other )) || builtin printf '\e]133;D;%s\a' $ret
        fi
        precmd_functions=(_paddock_marks_precmd ${precmd_functions:#_paddock_marks_(precmd|prompt)} _paddock_marks_prompt)
        return ret
    }
    _paddock_marks_bare() {
        _paddock_marks_left=${PS1//$'%{\e]133;A;paddock=1\a%}'/}
        _paddock_marks_left=${_paddock_marks_left//$'%{\e]133;B;paddock=1\a%}'/}
        [[ $_paddock_marks_left == *']133;'* ]] && _paddock_marks_other=1
    }
    _paddock_marks_prompt() {
        local ret=$?
        _paddock_marks_bare
        if (( _paddock_marks_other )); then
            PS1=$_paddock_marks_left
        elif [[ -o promptpercent ]]; then
            PS1=$'%{\e]133;A;paddock=1\a%}'$_paddock_marks_left$'%{\e]133;B;paddock=1\a%}'
        fi
        return ret
    }
    _paddock_marks_preexec() {
        _paddock_marks_running=1
        _paddock_marks_bare
        (( _paddock_marks_other )) || builtin printf '\e]133;C\a'
    }
    precmd_functions=(_paddock_marks_precmd ${precmd_functions:#_paddock_marks_(precmd|prompt)} _paddock_marks_prompt)
    preexec_functions=(${preexec_functions:#_paddock_marks_preexec} _paddock_marks_preexec)
fi
"#;

/// Run at the end of the private rcfile, once `~/.bashrc` has been read. The output starts where
/// bash prints `PS0` (bash 4.4 and later); older bash marks it from bash-preexec's hook when that
/// is loaded, or else from a DEBUG trap when nothing else has one.
pub const BASH: &str = r#"if [[ $- == *i* && -z "${_paddock_marks-}" ]]; then
    _paddock_marks=1
    _paddock_marks_other=0
    _paddock_marks_waiting=0
    _paddock_marks_ps0=0
    (( BASH_VERSINFO[0] > 4 || (BASH_VERSINFO[0] == 4 && BASH_VERSINFO[1] >= 4) )) && _paddock_marks_ps0=1
    _paddock_marks_status() {
        local ret=$?
        _paddock_marks_waiting=0
        [[ $_paddock_marks_other == 1 ]] || builtin printf '\e]133;D;%s\a' "$ret"
        return $ret
    }
    _paddock_marks_prompt() {
        local ret=$? a='\[\e]133;A;paddock=1\a\]' b='\[\e]133;B;paddock=1\a\]' c=$'\e]133;C;paddock=1\a' bare
        bare=${PS1//"$a"/}
        bare=${bare//"$b"/}
        [[ $bare == *']133;'* ]] && _paddock_marks_other=1
        if [[ $_paddock_marks_other == 1 ]]; then
            PS1=$bare
            [[ $_paddock_marks_ps0 == 1 ]] && PS0=${PS0//"$c"/}
        else
            PS1=$a$bare$b
            if [[ $_paddock_marks_ps0 == 1 ]]; then
                [[ ${PS0-} == "$c"* ]] || PS0=$c${PS0-}
            else
                _paddock_marks_waiting=1
            fi
        fi
        return $ret
    }
    _paddock_marks_preexec() {
        if [[ $_paddock_marks_waiting == 1 ]]; then
            _paddock_marks_waiting=0
            builtin printf '\e]133;C\a'
        fi
    }
    _paddock_marks_debug() {
        [[ -n ${COMP_LINE-} || -n ${READLINE_LINE-} || $BASH_COMMAND == _paddock_marks_status ]] || _paddock_marks_preexec
    }
    if [[ $_paddock_marks_ps0 == 0 ]]; then
        if [[ -n ${bash_preexec_imported-}${__bp_imported-} ]]; then
            preexec_functions+=(_paddock_marks_preexec)
        elif [[ -z "$(trap -p DEBUG)" ]]; then
            trap '_paddock_marks_debug' DEBUG
        fi
    fi
    if [[ "$(declare -p PROMPT_COMMAND 2>/dev/null)" == 'declare -a'* ]]; then
        PROMPT_COMMAND=(_paddock_marks_status "${PROMPT_COMMAND[@]}" _paddock_marks_prompt)
    else
        PROMPT_COMMAND=_paddock_marks_status$'\n'"${PROMPT_COMMAND:+$PROMPT_COMMAND$'\n'}"_paddock_marks_prompt
    fi
fi
"#;

/// Puts back the `ZDOTDIR` the shell was started with (unset stays unset) and reads the user's
/// own `.zshenv`, before [`ZSH`].
const ZSHENV: &str = r#"if [[ "$PADDOCK_SHELL_ZDOTDIR_SET" == 1 ]]; then
    export ZDOTDIR=$PADDOCK_SHELL_ZDOTDIR
else
    unset ZDOTDIR
fi
unset PADDOCK_SHELL_ZDOTDIR PADDOCK_SHELL_ZDOTDIR_SET
[[ -r "${ZDOTDIR-$HOME}/.zshenv" ]] && source "${ZDOTDIR-$HOME}/.zshenv"
"#;

const BASHRC: &str = "[[ -r \"$HOME/.bashrc\" ]] && source \"$HOME/.bashrc\"\n";

/// A plain zsh or bash (`program`) that marks its commands: its private startup file goes in
/// `dir`, kept to its owner. `original_zdotdir`: the `ZDOTDIR` the shell would have had.
pub fn setup(program: &str, dir: &Path, original_zdotdir: Option<&str>) -> Result<Setup> {
    anyhow::ensure!(
        crate::agent_shell::supported(program),
        "command marks support only zsh and bash"
    );
    let dir = crate::control::prepare_runtime(dir)?;
    let mut command = vec![program.to_owned()];
    let mut env = Vec::new();
    if is_zsh(program) {
        write_private(&dir, ".zshenv", &[ZSHENV, ZSH])?;
        env.push(("ZDOTDIR".into(), dir.to_string_lossy().into_owned()));
        env.push((
            "PADDOCK_SHELL_ZDOTDIR_SET".into(),
            if original_zdotdir.is_some() { "1" } else { "0" }.into(),
        ));
        env.push((
            "PADDOCK_SHELL_ZDOTDIR".into(),
            original_zdotdir.unwrap_or_default().into(),
        ));
    } else {
        write_private(&dir, "bashrc", &[BASHRC, BASH])?;
        command.extend([
            "--rcfile".into(),
            dir.join("bashrc").to_string_lossy().into_owned(),
        ]);
    }
    command.push("-i".into());
    Ok(Setup { command, env })
}

pub(crate) fn is_zsh(program: &str) -> bool {
    Path::new(program).file_name().and_then(|s| s.to_str()) == Some("zsh")
}

/// Writes `parts` to `dir/name` readable only by its owner, replacing the file in one step.
pub(crate) fn write_private(dir: &Path, name: &str, parts: &[&str]) -> Result<()> {
    let temporary = dir.join(crate::control::random_id()?);
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&temporary)?;
    for part in parts {
        file.write_all(part.as_bytes())?;
    }
    std::fs::rename(temporary, dir.join(name))?;
    Ok(())
}
