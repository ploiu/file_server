select id, name
from FileRecords
where lower(name) like ?1