-- insert workspaces

INSERT INTO workspaces (name, owner_id)
VALUES ('acme', 0), ('beta', 0), ('gamma', 0);

-- insert users
INSERT INTO users (ws_id, email, fullname, password_hash)
VALUES (1, 'zzq@zzq.com', 'zzq', '$argon2id$v=19$m=19456,t=2,p=1$6oJfE3UiponQts0znvyM4g$MLggmVmAxtLBi7uqAh6LSTotu2isSpYsCnO660jbfdE'),
(1, 'a@zzq.com', 'a', '$argon2id$v=19$m=19456,t=2,p=1$6oJfE3UiponQts0znvyM4g$MLggmVmAxtLBi7uqAh6LSTotu2isSpYsCnO660jbfdE'),
(1, 'b@zzq.com', 'b', '$argon2id$v=19$m=19456,t=2,p=1$6oJfE3UiponQts0znvyM4g$MLggmVmAxtLBi7uqAh6LSTotu2isSpYsCnO660jbfdE'),
(1, 'c@zzq.com', 'c', '$argon2id$v=19$m=19456,t=2,p=1$6oJfE3UiponQts0znvyM4g$MLggmVmAxtLBi7uqAh6LSTotu2isSpYsCnO660jbfdE'),
(1, 'd@zzq.com', 'd', '$argon2id$v=19$m=19456,t=2,p=1$6oJfE3UiponQts0znvyM4g$MLggmVmAxtLBi7uqAh6LSTotu2isSpYsCnO660jbfdE');


insert into chats (ws_id, name, type, members)
VALUES (1, 'general', 'public_channel', '{1,2,3,4,5}'),
(1, 'private', 'private_channel', '{1,2,3}');


-- insert unnamed chat
INSERT INTO chats (ws_id, type, members)
VALUES (1, 'single', '{1,2}'),
(1, 'group', '{1,2,3}');

INSERT INTO messages (chat_id, sender_id, content)
VALUES (1, 1, 'hello'),
(1, 2, 'world'),
(1, 3, 'zzq'),
(1, 4, 'a'),
(1, 5, 'b'),
(1, 1, 'hello'),
(1, 2, 'world'),
(1, 3, 'zzq'),
(1, 1, 'hello'),
(1, 2, 'world');
