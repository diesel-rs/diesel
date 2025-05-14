ALTER TABLE posts DROP CONSTRAINT posts_user_id_fkey;
ALTER TABLE comments DROP CONSTRAINT comments_post_id_fkey;
ALTER TABLE followings DROP CONSTRAINT followings_user_id_fkey;
ALTER TABLE followings DROP CONSTRAINT followings_post_id_fkey;
ALTER TABLE likes DROP CONSTRAINT likes_comment_id_fkey;
ALTER TABLE likes DROP CONSTRAINT likes_user_id_fkey;
