-- Remove implicit tags from folders where the tag is inherited from a specific folder
delete from TaggedItems
where folderId in (?1)
  and tagId = ?1
  and implicitFromId = ?2
