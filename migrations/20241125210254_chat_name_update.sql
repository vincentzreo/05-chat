-- Add migration script here
-- 如果chat name发生变化，发送通知
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

CREATE TRIGGER notify_chat_name_change_trigger
AFTER
UPDATE
    ON chats FOR EACH ROW EXECUTE FUNCTION notify_chat_name_change();
