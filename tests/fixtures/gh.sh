#!/bin/sh
printf '%s\n' "$*" >> "$TEST_MGH_ROOT/gh-calls"

case "$1 $2" in
    'auth login')
        printf 'GitHub browser login\n'
        [ -f "$TEST_MGH_ROOT/fail-login" ] && exit 1
        if [ -f "$TEST_MGH_ROOT/login-accounts-json" ]; then
            cp "$TEST_MGH_ROOT/login-accounts-json" "$TEST_MGH_ROOT/accounts-json"
        fi
        if [ -f "$TEST_MGH_ROOT/login-accounts-conf" ]; then
            cp "$TEST_MGH_ROOT/login-accounts-conf" "$XDG_CONFIG_HOME/multigh/accounts.conf"
        fi
        ;;
    'config get')
        [ -f "$TEST_MGH_ROOT/fail-selected" ] && exit 1
        if [ -f "$TEST_MGH_ROOT/selected" ]; then
            cat "$TEST_MGH_ROOT/selected"
        else
            cat "$TEST_MGH_ROOT/active"
        fi
        ;;
    'auth switch')
        [ -f "$TEST_MGH_ROOT/fail-switch" ] && exit 1
        while [ "$1" != '--user' ]; do shift; done
        printf '%s\n' "$2" > "$TEST_MGH_ROOT/active"
        ;;
    'auth status')
        [ -f "$TEST_MGH_ROOT/fail-auth" ] && exit 1
        case " $* " in
            *' --active '*) cat "$TEST_MGH_ROOT/active" ;;
            *' --json '*)
                if [ -f "$TEST_MGH_ROOT/accounts-json" ]; then
                    cat "$TEST_MGH_ROOT/accounts-json"
                else
                    alice=false; bob=false
                    case "$(cat "$TEST_MGH_ROOT/active")" in
                        alice) alice=true ;;
                        bob) bob=true ;;
                    esac
                    printf '[{"login":"alice","active":%s,"state":"success"},{"login":"bob","active":%s,"state":"success"}]\n' "$alice" "$bob"
                fi
                ;;
            *)
                [ -f "$TEST_MGH_ROOT/fail-full" ] && exit 1
                printf 'GitHub authentication details\n'
                ;;
        esac
        ;;
    *) exit 1 ;;
esac
