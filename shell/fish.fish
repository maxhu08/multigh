function __mgh_enter --on-variable PWD
    if status is-interactive; and command -sq mgh
        command mgh enter 0<&1
    end
end

if status is-interactive
    __mgh_enter
end
