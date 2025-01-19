import init, {
    init_sqlite,
    switch_vfs,
    create_post,
    get_post,
    delete_post,
    publish_post,
    show_posts
} from "./pkg/sqlite_wasm_example.js";

await init();
await init_sqlite();

async function run_in_worker(event) {
    const payload = event.data;
    switch (payload.cmd) {
        case 'switch_vfs':
            switch_vfs(payload.id);
            break;
        case 'show_posts':
            var posts = show_posts();
            self.postMessage(
                {
                    cmd: 'show_posts',
                    posts: posts
                }
            );
            break;
        case 'create_post':
            var post = create_post(payload.title, payload.body);
            self.postMessage(
                {
                    cmd: 'create_post',
                    post: post,
                }
            );
            break;
        case 'get_post':
            var post = get_post(payload.post_id);
            self.postMessage(
                {
                    cmd: 'get_post',
                    post: post,
                }
            );
            break;
        case 'publish_post':
            publish_post(payload.post_id);
            self.postMessage(
                {
                    cmd: 'publish_post',
                }
            );
            break;
        case 'delete_post':
            delete_post(payload.title);
            self.postMessage(
                {
                    cmd: 'delete_post',
                }
            );
            break;
        default:
            break;
    };
}

self.onmessage = function (event) {
    run_in_worker(event);
}

self.postMessage("ready");
