-- biomedgps_ai_message table is used to store the messages which are generated by the AI system, such as ChatGPT, BioBERT, etc.

CREATE TABLE
  IF NOT EXISTS biomedgps_ai_message (
    id BIGSERIAL PRIMARY KEY, -- The entity metadata ID
    session_uuid VARCHAR(64) NOT NULL UNIQUE, -- The UUID of the session which is used to generate the message
    prompt_template TEXT NOT NULL, -- The prompt template which is used to generate the message
    prompt_template_category VARCHAR(64) NOT NULL, -- The category of the prompt template which is used to generate the message
    context JSONB NOT NULL, -- The context which is used to generate the message
    prompt TEXT NOT NULL, -- The prompt which is used to generate the message
    message TEXT NOT NULL, -- The message which is generated by the AI system
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP, -- The created time of the message
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP, -- The updated time of the message
    CONSTRAINT biomedgps_ai_message_uniq_key UNIQUE (uuid)
  );