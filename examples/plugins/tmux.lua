-- Opens/attaches a tmux session named after the clone
repoman.on("post_clone", function(ctx)
    local session = ctx.clone_name:gsub("[%.%-]", "_")
    repoman.exec("tmux new-session -d -s " .. session .. " -c " .. ctx.clone_path)
    repoman.log("info", "tmux session '" .. session .. "' created")
end)

repoman.on("pre_destroy", function(ctx)
    local session = ctx.clone_name:gsub("[%.%-]", "_")
    repoman.exec("tmux kill-session -t " .. session .. " 2>/dev/null || true")
end)
