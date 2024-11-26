-- Add migration script here
-- if chat changed, notify with chat data
CREATE
OR REPLACE FUNCTION add_to_chat() RETURNS TRIGGER AS $$ BEGIN RAISE NOTICE 'add_to_chat: %',
NEW;

PERFORM pg_notify(
    'chat_updated',
    json_build_object(
        'op',
        TG_OP,
        'old',
        OLD,
        'new',
        NEW
    ) :: text
);

RETURN NEW;

END;

$$ LANGUAGE plpgsql;

CREATE TRIGGER add_to_chat_trigger
AFTER
INSERT
    OR
UPDATE
    OR DELETE ON chats FOR EACH ROW EXECUTE FUNCTION add_to_chat();

-- if new message added, notify with message data
CREATE
OR REPLACE FUNCTION add_to_message() RETURNS TRIGGER AS $$ DECLARE USERS bigint [];

BEGIN IF TG_OP = 'INSERT' THEN RAISE NOTICE 'add_to_message: %',
NEW;

SELECT
    members INTO USERS
FROM
    chats
WHERE
    id = NEW.chat_id;

PERFORM pg_notify(
    'chat_message_added',
    json_build_object(
        'message',
        NEW,
        'members',
        USERS
    ) :: text
);

END IF;

RETURN NEW;

END;

$$ LANGUAGE plpgsql;

CREATE TRIGGER add_to_message_trigger
AFTER
INSERT
    ON messages FOR EACH ROW EXECUTE FUNCTION add_to_message();

CREATE
OR REPLACE FUNCTION notify_chat_name_change() RETURNS TRIGGER AS $$ BEGIN IF (
    TG_OP = 'UPDATE'
    AND OLD.name IS DISTINCT
    FROM
        NEW.name
) THEN PERFORM pg_notify(
    'chat_name_updated',
    json_build_object(
        'chat_id',
        NEW.id,
        'old_name',
        OLD.name,
        'new_name',
        NEW.name,
        'members',
        NEW.members
    ) :: text
);

END IF;

RETURN NEW;

END;

$$ LANGUAGE plpgsql;
