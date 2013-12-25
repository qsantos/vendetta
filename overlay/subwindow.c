#include "subwindow.h"

void subwindow_init(subwindow_t* w, const wchar_t* name, float x, float y)
{
	w->name = name;
	w->x = x;
	w->y = y;
	w->visible = 1;
}

void subwindow_exit(subwindow_t* w)
{
	(void) w;
}

char subwindow_draw(subwindow_t* w, graphics_t* g)
{
	if (!w->visible)
		return 0;

	static sfSprite* sprite = NULL;
	if (sprite == NULL)
	{
		int id = graphics_spriteForImg(g, "subwindow.png");
		sprite = g->sprites[id];
	}
	sfVector2f pos = {w->x, w->y};
	sfSprite_setPosition(sprite, pos);
	sfRenderWindow_drawSprite(g->render, sprite, NULL);

	static sfText* text = NULL;
	if (text == NULL)
	{
		text = sfText_create();
		sfColor col = {255, 255, 255, 255};
		sfText_setColor        (text, col);
		sfText_setFont         (text, g->font);
		sfText_setCharacterSize(text, 18);
	}
	sfText_setUnicodeString(text, (sfUint32*) w->name);

	sfFloatRect rect = sfText_getLocalBounds(text);
	pos.x += (SW_WIDTH-rect.width)/2;
	pos.y += 20;
	sfText_setPosition(text, pos);

	sfRenderWindow_drawText(g->render, text, NULL);

	return 1;
}
