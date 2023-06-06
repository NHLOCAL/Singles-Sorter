document.getElementById('searchForm').addEventListener('submit', function(event) {
  event.preventDefault();
  var searchInput = document.getElementById('searchInput').value.toLowerCase();
  var searchBy = document.getElementById('searchBy').value;
  searchSongs(searchInput, searchBy);
});

function searchSongs(query, searchBy) {
  fetch('songs.csv')
    .then(function(response) {
      return response.text();
    })
    .then(function(csvText) {
      var songs = parseCSV(csvText);
      var results = filterSongs(songs, query, searchBy);
      displayResults(results);
    });
}

function parseCSV(csvText) {
  var lines = csvText.split('\n');
  var songs = [];
  for (var i = 1; i < lines.length; i++) {
    var line = lines[i].trim();
    if (line) {
      var columns = line.split(/,(?=(?:(?:[^"]*"){2})*[^"]*$)/);
      var song = {
        serial: columns[0].trim(),
        name: columns[1].trim(),
        album: columns[2].trim(),
        singer: columns[3].trim()
      };
      songs.push(song);
    }
  }
  return songs;
}

function filterSongs(songs, query, searchBy) {
  return songs.filter(function(song) {
    var values = Object.values(song).map(function(value) {
      return value.toLowerCase();
    });
    return values.some(function(value) {
      return value.includes(query);
    });
  });
}


function displayResults(results) {
  var tableBody = document.querySelector('#resultsTable tbody');
  tableBody.innerHTML = '';
  for (var i = 0; i < results.length; i++) {
    var song = results[i];
    var row = document.createElement('tr');
    var serialCell = document.createElement('td');
    var nameCell = document.createElement('td');
    var albumCell = document.createElement('td');
    var singerCell = document.createElement('td');
    serialCell.textContent = song.serial;
    nameCell.textContent = song.name;
    albumCell.textContent = song.album;
    singerCell.textContent = song.singer;
    row.appendChild(serialCell);
    row.appendChild(nameCell);
    row.appendChild(albumCell);
    row.appendChild(singerCell);
    tableBody.appendChild(row);
  }
}
