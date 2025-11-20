-- Remove implicit tags from files where the tag is inherited from a specific folder
delete from TaggedItems
where fileId in (?1)
  and tagId = ?1
  and implicitFromId = ?2
