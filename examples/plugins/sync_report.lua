-- Logs sync events to a weekly report file
repoman.on("post_sync", function(ctx)
    local report = os.getenv("HOME") .. "/.repoman/logs/sync_report.log"
    local f = io.open(report, "a")
    if f then
        f:write(os.date("%Y-%m-%d %H:%M:%S") .. " | " .. ctx.repo .. " | synced\n")
        f:close()
    end
end)

repoman.on("post_sync_on_new_tag", function(ctx)
    local report = os.getenv("HOME") .. "/.repoman/logs/sync_report.log"
    local f = io.open(report, "a")
    if f then
        f:write(os.date("%Y-%m-%d %H:%M:%S") .. " | " .. ctx.repo .. " | new tag: " .. (ctx.new_tag or "unknown") .. "\n")
        f:close()
    end
end)
