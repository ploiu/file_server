-- Get files from a list that don't have a specific tag (explicit or implicit)
-- Input: file IDs as a formatted IN clause string (?1), and tag ID (?2)
select id
from FileRecords
where id in ({})
  and id not in (
      select fileId
      from TaggedItems
      where tagId = ?
        and fileId is not null
  )
