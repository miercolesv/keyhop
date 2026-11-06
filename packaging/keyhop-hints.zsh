# keyhop terminal hints: after every keystroke, tell keyhop which characters
# can come next in the word being typed, so the keyboard lights those keys.
# Candidates come from zsh's own completion system (what Tab would offer), run
# once per word and narrowed locally while typing. Without compinit, falls
# back to command names and files. Needs `terminal_hints = on` in keyhop.conf.
#
# `keyhop setup` installs this file and sources it from ~/.zshrc (after
# compinit / oh-my-zsh, so Tab's completions are available).

zmodload zsh/net/socket 2>/dev/null || return
autoload -Uz add-zle-hook-widget || return

typeset -g _keyhop_sock=${XDG_CONFIG_HOME:-$HOME/.config}/keyhop/hints.sock
typeset -ga _keyhop_matches
typeset -g _keyhop_key=$'\0' _keyhop_frag= _keyhop_busy=

# Send one line to keyhop; silently does nothing when keyhop isn't running.
_keyhop_send() {
  [[ -S $_keyhop_sock ]] || return 0
  zsocket $_keyhop_sock 2>/dev/null || return 0
  local fd=$REPLY
  print -r -u $fd -- "$1"
  exec {fd}>&-
}

# Completion widget: run Tab's completion, but collect the matches (compadd -O)
# instead of inserting or listing anything.
_keyhop_capture() {
  _keyhop_matches=()
  compadd() {
    local -a m
    builtin compadd -O m "$@"
    _keyhop_matches+=("${m[@]}")
  }
  { _main_complete } always { unfunction compadd }
  compstate[insert]=''
  compstate[list]=''
}

# Without compinit: command names for the first word, files after that.
_keyhop_fallback() {
  local word=$1 first=$2 pat
  if (( first )); then
    pat=${(b)word}*
    _keyhop_matches=(${(k)commands[(I)$pat]} ${(k)builtins[(I)$pat]} ${(k)aliases[(I)$pat]} ${(k)functions[(I)$pat]})
  else
    pat=$word
    [[ $pat == '~'* ]] && pat=$HOME${pat#\~}
    _keyhop_matches=(${(b)pat}*(N-M[1,500]))
    _keyhop_matches=(${_keyhop_matches##*/})
  fi
}

_keyhop_hint() {
  [[ -n $_keyhop_busy ]] && return
  local word frag key m next
  local -a words=(${(z)LBUFFER})
  # The word under the cursor (empty right after a space) and its last path part.
  if [[ -z $LBUFFER || $LBUFFER == *[[:space:]] ]]; then
    word=
  else
    word=${words[-1]}
  fi
  frag=${word##*/}
  # Nothing at an empty prompt: every letter would match.
  [[ -z ${LBUFFER//[[:space:]]/} ]] && { _keyhop_send ''; return }
  # Re-run completion when a new word (or path part) starts, and again on its
  # first character (options only complete after "-", for example).
  key=${LBUFFER%$frag}
  if [[ $key != $_keyhop_key || $frag != $_keyhop_frag* || ( -z $_keyhop_frag && -n $frag ) ]]; then
    _keyhop_key=$key _keyhop_frag=$frag
    if (( $+functions[_main_complete] )); then
      _keyhop_busy=1
      zle _keyhop_complete 2>/dev/null
      _keyhop_busy=
    else
      _keyhop_fallback "$word" $(( ${#words} <= 1 && ${#word} > 0 ))
    fi
  fi
  # Next character of every candidate that continues what was typed.
  next=
  for m in $_keyhop_matches; do
    [[ ${m:l} == ${frag:l}* ]] || continue
    next+=${m[${#frag}+1]}
  done
  _keyhop_send "$next"
}

_keyhop_clear() {
  _keyhop_key=$'\0'
  _keyhop_send ''
}

zle -C _keyhop_complete complete-word _keyhop_capture
add-zle-hook-widget line-pre-redraw _keyhop_hint
add-zle-hook-widget line-finish _keyhop_clear
