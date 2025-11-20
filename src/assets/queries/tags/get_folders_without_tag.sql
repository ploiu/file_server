-- Get folders from a list that don't have a specific tag (explicit or implicit)
-- Input: folder IDs as a formatted IN clause string (?1), and tag ID (?2)
select id
from Folders
where id in ({})
  and id not in (
      select folderId
      from TaggedItems
      where tagId = ?
        and folderId is not null
  )
