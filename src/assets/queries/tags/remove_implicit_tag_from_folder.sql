delete from TaggedItems
where folderId = ?1
  and tagId = ?2
  and implicitFromId is not null
