import init, {
    installOpfsSahpool,
    installRelaxedIdb,
    switchVfs,
    createPost,
    getPost,
    deletePost,
    publishPost,
    showPosts
} from './pkg/sqlite_wasm_example.js';

// Initialize WASM module
await init();
await installOpfsSahpool();
await installRelaxedIdb();

/**
 * Handles incoming messages from the main thread
 * @param {MessageEvent} event - The message event
 */
async function handleMessage(event) {
    const { cmd, ...payload } = event.data;

    try {
        switch (cmd) {
            case 'switch_vfs':
                switchVfs(payload.id);
                break;

            case 'show_posts': {
                const posts = await showPosts();
                postResponse(cmd, { posts });
                break;
            }

            case 'create_post': {
                const { title, body } = payload;
                if (!title || !body) {
                    throw new Error('Title and body are required');
                }
                const post = await createPost(title, body);
                postResponse(cmd, { post });
                break;
            }

            case 'get_post': {
                const { post_id: postId } = payload;
                if (!postId) {
                    throw new Error('Post ID is required');
                }
                const post = await getPost(postId);
                postResponse(cmd, { post });
                break;
            }

            case 'publish_post': {
                const { post_id: postId } = payload;
                if (!postId) {
                    throw new Error('Post ID is required');
                }
                await publishPost(postId);
                postResponse(cmd);
                break;
            }

            case 'delete_post': {
                const { title } = payload;
                if (!title) {
                    throw new Error('Title is required');
                }
                await deletePost(title);
                postResponse(cmd);
                break;
            }

            default:
                console.warn(`Unknown command: ${cmd}`);
                postResponse('error', { error: `Unknown command: ${cmd}` });
        }
    } catch (error) {
        console.error(`Error processing ${cmd}:`, error);
        postResponse('error', {
            error: error.message,
            originalCmd: cmd
        });
    }
}

/**
 * Posts a response back to the main thread
 * @param {string} cmd - The command name
 * @param {object} [data] - Additional response data
 */
function postResponse(cmd, data = {}) {
    self.postMessage({
        cmd,
        ...data
    });
}

// Set up message listener
self.addEventListener('message', handleMessage);

// Notify main thread that worker is ready
postResponse('ready');
