ALTER TABLE posts ADD CONSTRAINT posts_user_id_fkey FOREIGN KEY (user_id) REFERENCES users DEFERRABLE;
ALTER TABLE comments ADD CONSTRAINT comments_post_id_fkey FOREIGN KEY (post_id) REFERENCES posts DEFERRABLE;
ALTER TABLE followings ADD CONSTRAINT followings_user_id_fkey FOREIGN KEY (user_id) REFERENCES users DEFERRABLE;
ALTER TABLE followings ADD CONSTRAINT followings_post_id_fkey FOREIGN KEY (post_id) REFERENCES posts DEFERRABLE;
ALTER TABLE likes ADD CONSTRAINT likes_comment_id_fkey FOREIGN KEY (comment_id) REFERENCES comments DEFERRABLE;
ALTER TABLE likes ADD CONSTRAINT likes_user_id_fkey FOREIGN KEY (user_id) REFERENCES users DEFERRABLE;
