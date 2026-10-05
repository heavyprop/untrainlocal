// this is the script for loading more posts without reloading the page
// detects button click, requests the next batch from django to stop querrying whole db 
// since we did alot of querrying already in the db
// appends the returned HTML below the existing posts
// updates the offset and hides the button when results run out
(() => {
    const button = document.getElementById("search-load-more");
    if (!button) return;

    const results = document.getElementById("search-results");
    const count = document.getElementById("search-result-count");
    const status = document.getElementById("search-status");

    let loading = false;
    let finished = false;

    button.addEventListener("click", async () => {
        if (loading || finished) return;

        loading = true;
        button.disabled = true;
        button.textContent = "Loading...";
        status.textContent = "";
        results.setAttribute("aria-busy", "true");

        try {
            const url = new URL(button.dataset.url, window.location.origin);
            url.searchParams.set("offset", button.dataset.offset);

            const response = await fetch(url, {
                credentials: "same-origin",
                headers: { Accept: "application/json" },
            });

            const contentType = response.headers.get("content-type") || "";

            if (!contentType.includes("application/json")) {
                throw new Error("Unable to load results. Try refreshing the page");
            }

            const data = await response.json();

            if (!response.ok) {
                if (response.status === 401 || response.status === 410) {
                    finished = true;
                    button.hidden = true;
                }

                throw new Error(data.error || "Unable to laod more results.");
            }

            results.insertAdjacentHTML("beforeend", data.html);
            count.textContent = String(Number(count.textContent) + data.count);

            if (data.has_more) {
                button.dataset.offset = String(data.next_offset);
                status.textContent = `Loaded ${data.count} more results.`;
            } else {
                finished = true;
                button.hidden = true;
                status.textContent = "All results loaded.";
            }
        } catch (error) {
            status.textContent = error.message || "Unable to load more results.";
        } finally {
            loading = false;
            button.disabled = false;
            button.textContent = "Load more";
            results.removeAttribute("aria-busy");
        }
    });
})();
