# multigh interactive shell integration
if status is-interactive
    if not set -q _MGH_FISH_LOADED
        set -g _MGH_FISH_LOADED 1
        mgh shell completions fish | source
        function _mgh_enter --on-variable PWD
            if not set -q _MGH_LAST_PWD; or test "$_MGH_LAST_PWD" != "$PWD"
                set -g _MGH_LAST_PWD "$PWD"
                mgh internal enter </dev/tty
            end
        end
        if not functions -q git
            function git
                mgh internal git -- $argv
            end
        end
        function _mgh_startup --on-event fish_prompt
            functions --erase _mgh_startup
            mgh internal welcome </dev/tty
            _mgh_enter </dev/tty
        end
    end
end
