package com.nhlocal.singlesorter.app;

import org.json.JSONArray;
import org.json.JSONObject;

/** Display units come from verified subtree evidence, never from the number of direct children. */
final class AlbumUiProjection {
    private AlbumUiProjection(){}
    static long files(JSONObject album)throws Exception{return sum(album,"files");}
    static long directories(JSONObject album)throws Exception{return sum(album,"directories");}
    static long movedFiles(JSONObject album)throws Exception{return album.getBoolean("copy")?0:files(album);}
    private static long sum(JSONObject album,String unit)throws Exception{long count=0;JSONArray children=album.getJSONArray("children");for(int i=0;i<children.length();i++){JSONObject child=children.getJSONObject(i);if("complete".equals(child.getString("status")))count+=child.getJSONObject("tree_binding").getLong(unit);}return count;}
    static JSONObject summary(JSONObject[] albums,String[] status)throws Exception{
        int completed=0,renamed=0,retained=0,created=0,removed=0,directChildren=0;long files=0,directories=0;
        for(int i=0;i<albums.length;i++){JSONObject album=albums[i];if(album==null)continue;files+=files(album);directories+=directories(album);created+=album.getInt("created_directories");removed+=album.getInt("removed_directories");if(!album.getBoolean("copy")&&!album.getBoolean("source_removed"))retained++;if("complete".equals(status[i])&&!album.isNull("destination"))completed++;
            JSONArray children=album.getJSONArray("children");for(int n=0;n<children.length();n++){JSONObject child=children.getJSONObject(n);if("complete".equals(child.getString("status"))){directChildren++;if(child.getBoolean("name_changed"))renamed++;}}}
        return new JSONObject().put("completed",completed).put("child_names_changed",renamed).put("created_directories",created).put("removed_directories",removed).put("retained_directories",retained).put("published_direct_children",directChildren).put("published_files",files).put("published_directories",directories);
    }
}
