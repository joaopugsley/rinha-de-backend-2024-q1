CREATE UNLOGGED TABLE client (
    id SERIAL PRIMARY KEY,
    limite INT NOT NULL,
    saldo INT NOT NULL DEFAULT 0
) WITH (autovacuum_enabled = false);

CREATE UNLOGGED TABLE transaction (
    id SERIAL PRIMARY KEY,
    client_id INT REFERENCES client(id) NOT NULL,
    valor INT NOT NULL,
    tipo CHAR(1) NOT NULL,
    descricao VARCHAR(10) NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
) WITH (autovacuum_enabled = false);

CREATE INDEX idx_transaction_client_id ON transaction (client_id);

INSERT INTO
    client (id, limite, saldo)
VALUES
    (1, 100000, 0),
    (2, 80000, 0),
    (3, 1000000, 0),
    (4, 10000000, 0),
    (5, 500000, 0);