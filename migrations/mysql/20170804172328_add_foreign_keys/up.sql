ALTER TABLE posts ADD CONSTRAINT posts_user_id_fkey FOREIGN KEY (user_id) REFERENCES users (id);
ALTER TABLE comments ADD CONSTRAINT comments_post_id_fkey FOREIGN KEY (post_id) REFERENCES posts (id);
ALTER TABLE followings ADD CONSTRAINT followings_user_id_fkey FOREIGN KEY (user_id) REFERENCES users (id);
ALTER TABLE followings ADD CONSTRAINT followings_post_id_fkey FOREIGN KEY (post_id) REFERENCES posts (id);
ALTER TABLE likes ADD CONSTRAINT likes_comment_id_fkey FOREIGN KEY (comment_id) REFERENCES comments (id);
ALTER TABLE likes ADD CONSTRAINT likes_user_id_fkey FOREIGN KEY (user_id) REFERENCES users (id);
