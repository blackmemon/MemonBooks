CREATE TABLE workflow_documents (
 id INTEGER PRIMARY KEY, company_id INTEGER NOT NULL REFERENCES companies(id),
 kind TEXT NOT NULL, number TEXT, status TEXT NOT NULL,
 transaction_id INTEGER REFERENCES transactions(id), revision INTEGER NOT NULL DEFAULT 0,
 created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
 modified_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
 UNIQUE(company_id,number)
);
CREATE TABLE workflow_fields(document_id INTEGER NOT NULL REFERENCES workflow_documents(id) ON DELETE CASCADE,row_no INTEGER NOT NULL,name TEXT NOT NULL,value TEXT NOT NULL,PRIMARY KEY(document_id,row_no,name));
CREATE TABLE workflow_attachments(id INTEGER PRIMARY KEY,document_id INTEGER NOT NULL REFERENCES workflow_documents(id) ON DELETE CASCADE,path TEXT NOT NULL);
CREATE TABLE workflow_templates(id INTEGER PRIMARY KEY,company_id INTEGER NOT NULL REFERENCES companies(id),kind TEXT NOT NULL,name TEXT NOT NULL,UNIQUE(company_id,kind,name));
CREATE TABLE workflow_template_fields(template_id INTEGER NOT NULL REFERENCES workflow_templates(id) ON DELETE CASCADE,row_no INTEGER NOT NULL,name TEXT NOT NULL,value TEXT NOT NULL,PRIMARY KEY(template_id,row_no,name));
CREATE TABLE workflow_credits(id INTEGER PRIMARY KEY,company_id INTEGER NOT NULL REFERENCES companies(id),party_type TEXT NOT NULL,party_id INTEGER NOT NULL,transaction_id INTEGER NOT NULL REFERENCES transactions(id),amount TEXT NOT NULL);
CREATE TABLE workflow_allocations(document_id INTEGER REFERENCES workflow_documents(id),subledger TEXT NOT NULL,entry_id INTEGER NOT NULL,amount TEXT NOT NULL);
CREATE INDEX workflow_company_kind ON workflow_documents(company_id,kind);
