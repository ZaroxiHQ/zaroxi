export default function SearchPanel() {
  return (
    <div className="p-4 h-full flex flex-col">
      <div className="space-y-4">
        <div className="space-y-2">
          <div className="relative">
            <input
              type="text"
              placeholder="Search files, text, or symbols..."
              className="w-full px-2 py-1 text-xs border border-input rounded-lg bg-background focus:outline-none focus:ring-2 focus:ring-primary"
            />
            <div className="absolute right-2 top-2 text-xs text-muted-foreground">
              Ctrl+Shift+F
            </div>
          </div>
          <div className="flex gap-2">
            <button className="px-3 py-1.5 text-xs border border-border rounded hover:bg-hover-bg">
              Files
            </button>
            <button className="px-3 py-1.5 text-xs border border-border rounded hover:bg-hover-bg">
              Text
            </button>
            <button className="px-3 py-1.5 text-xs border border-border rounded hover:bg-hover-bg">
              Symbols
            </button>
          </div>
        </div>
        
        <div className="border-t border-border pt-4">
          <h4 className="text-sm font-medium mb-2">Recent Searches</h4>
          <div className="space-y-1">
            <div className="text-xs text-muted-foreground p-2 hover:bg-hover-bg rounded cursor-pointer">
              "useState" in workspace
            </div>
            <div className="text-xs text-muted-foreground p-2 hover:bg-hover-bg rounded cursor-pointer">
              function.*Component
            </div>
          </div>
        </div>
        
        <div className="border-t border-border pt-4">
          <h4 className="text-sm font-medium mb-2">Search Tips</h4>
          <ul className="text-xs text-muted-foreground space-y-1">
            <li>• Use <code className="bg-muted px-1 rounded">*</code> for wildcards</li>
            <li>• Use <code className="bg-muted px-1 rounded">path:</code> to search in specific folders</li>
            <li>• Press <code className="bg-muted px-1 rounded">Enter</code> to search</li>
          </ul>
        </div>
      </div>
      
      <div className="mt-auto pt-4 border-t border-border">
        <p className="text-xs text-muted-foreground text-center">
          Search across your entire workspace
        </p>
      </div>
    </div>
  );
}
