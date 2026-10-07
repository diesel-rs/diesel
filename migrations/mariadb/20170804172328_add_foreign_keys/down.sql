ALTER TABLE posts DROP FOREIGN KEY posts_user_id_fkey;
ALTER TABLE comments DROP FOREIGN KEY comments_post_id_fkey;
ALTER TABLE followings DROP FOREIGN KEY followings_user_id_fkey;
ALTER TABLE followings DROP FOREIGN KEY followings_post_id_fkey;
ALTER TABLE likes DROP FOREIGN KEY likes_comment_id_fkey;
ALTER TABLE likes DROP FOREIGN KEY likes_user_id_fkey;
