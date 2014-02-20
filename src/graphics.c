/*\
 *  Role playing, management and strategy game
 *  Copyright (C) 2013-2014 Quentin SANTOS
 *
 *  This program is free software: you can redistribute it and/or modify
 *  it under the terms of the GNU General Public License as published by
 *  the Free Software Foundation, either version 3 of the License, or
 *  (at your option) any later version.
 *
 *  This program is distributed in the hope that it will be useful,
 *  but WITHOUT ANY WARRANTY; without even the implied warranty of
 *  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 *  GNU General Public License for more details.
 *
 *  You should have received a copy of the GNU General Public License
 *  along with this program.  If not, see <http://www.gnu.org/licenses/>.
\*/

#include "graphics.h"

#include <stdlib.h>

#include "version.h"

void graphics_init(graphics_t* g)
{
	sfVideoMode mode = {1024, 768, 32};
	g->render = sfRenderWindow_create(mode, "Vendetta " VERSION, sfResize | sfClose, NULL);
	if (g->render == NULL)
		exit(1);
	sfRenderWindow_setMouseCursorVisible(g->render, sfFalse);
}

void graphics_exit(graphics_t* g)
{
	sfRenderWindow_destroy(g->render);
}

#ifdef __WIN32__
#include <windows.h>
#include <winnls.h>
#endif
void sfText_setUTF8(sfText* text, const char* string)
{
	sfUint32 buf32[1024];
#ifdef __WIN32__
	wchar_t buf16[1024];
	MultiByteToWideChar(CP_UTF8, 0, string, -1, buf16, 1024);
	size_t n = wcslen(buf16)+1;
	for (size_t i = 0; i < n; i++)
		buf32[i] = buf16[i];
#else
	mbstowcs((wchar_t*) buf32, string, 1024);
#endif
	sfText_setUnicodeString(text, buf32);
}

char sfText_contains(sfText* text, sfVector2f point)
{
	sfFloatRect rect = sfText_getGlobalBounds(text);
	return sfFloatRect_contains(&rect, point.x, point.y);
}

char sfSprite_contains(sfSprite* sprite, sfVector2f point)
{
	sfFloatRect rect = sfSprite_getGlobalBounds(sprite);
	return sfFloatRect_contains(&rect, point.x, point.y);
}
